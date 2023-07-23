use std::{
    collections::HashMap,
    net::SocketAddrV6,
    sync::{mpsc, Arc, Mutex},
};

use gtk::glib;

use crate::{
    error::CustomError,
    gui::init::GUIEvents,
    logger::{send_log, Log},
    message::Message,
    messages::{
        block::Block, get_data::GetData, get_headers::GetHeaders, headers::Headers, inv::Inv,
        not_found::NotFound, transaction::Transaction,
    },
    node_state::NodeState,
    structs::{
        block_header::{hash_as_string, BlockHeader},
        inventory::{Inventory, InventoryType},
    },
};

use super::peer_action_loop::PeerAction;

/// NodeAction es una enumeracion de las acciones que puede realizar el nodo.
/// Las acciones son:
/// - NewHeaders: Recibe nuevos headers.
/// - GetHeadersError: Error al solicitar headers.
/// - Block: Recibe un bloque.
/// - GetDataError: Error al solicitar data.
/// - PendingTransaction: Recibe una transaccion.
/// - MakeTransaction: Solicitar una transaccion.
pub enum NodeAction {
    PeerError(SocketAddrV6),
    NewHeaders(Headers),
    GetHeadersError,
    Block((Vec<u8>, Block)),
    GetDataError(Vec<Inventory>),
    PendingTransaction(Transaction),
    MakeTransaction((HashMap<String, u64>, u64)),
    SendHeaders(SocketAddrV6),
    GetHeaders(SocketAddrV6, GetHeaders),
    GetData(SocketAddrV6, GetData),
}

const START_DATE_IBD: u32 = 1681095630;

/// NodeActionLoop es una estructura que contiene los elementos necesarios para manejar los mensajes recibidos por el nodo.
/// Genera el loop de eventos alrededor de los NodeAction recibidoe por node_action_receiver.
/// Los elementos son:
/// - gui_sender: Sender para enviar eventos a la interfaz grafica.
/// - node_action_receiver: Receiver para recibir acciones del nodo.
/// - peer_action_sender: Sender para enviar acciones al los peers.
/// - logger_sender: Sender para enviar logs al logger.
/// - node_state_ref: Referencia al estado del nodo.
pub struct NodeActionLoop {
    gui_sender: glib::Sender<GUIEvents>,
    node_action_receiver: mpsc::Receiver<NodeAction>,
    peer_action_sender: mpsc::Sender<PeerAction>,
    logger_sender: mpsc::Sender<Log>,
    node_state_ref: Arc<Mutex<NodeState>>,
}

impl NodeActionLoop {
    /// Inicializa el loop de eventos.
    pub fn start(
        gui_sender: glib::Sender<GUIEvents>,
        node_action_receiver: mpsc::Receiver<NodeAction>,
        peer_action_sender: mpsc::Sender<PeerAction>,
        logger_sender: mpsc::Sender<Log>,
        node_state_ref: Arc<Mutex<NodeState>>,
    ) {
        let mut node_thread = Self {
            gui_sender,
            node_action_receiver,
            peer_action_sender,
            logger_sender,
            node_state_ref,
        };
        node_thread.event_loop();
    }

    fn event_loop(&mut self) {
        while let Ok(message) = self.node_action_receiver.recv() {
            let response = match message {
                NodeAction::PeerError(address) => self.handle_peer_error(address),
                NodeAction::Block((block_hash, block)) => self.handle_block(block_hash, block),
                NodeAction::NewHeaders(new_headers) => self.handle_new_headers(new_headers),
                NodeAction::GetHeadersError => self.handle_get_headers_error(),
                NodeAction::GetDataError(inventory) => self.handle_get_data_error(inventory),
                NodeAction::MakeTransaction((outputs, fee)) => {
                    self.handle_make_transaction(outputs, fee)
                }
                NodeAction::PendingTransaction(transaction) => {
                    self.handle_pending_transaction(transaction)
                }
                NodeAction::SendHeaders(address) => self.handle_send_headers(address),
                NodeAction::GetHeaders(address, getheaders) => {
                    self.handle_get_headers(address, getheaders)
                }
                NodeAction::GetData(address, getdata) => self.handle_get_data(address, getdata),
                // NodeAction::TestInv(inv) => self.handle_test_inv(inv),
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on NodeActionLoop: {error}")),
                );
            }
        }
    }

    fn handle_peer_error(&mut self, address: SocketAddrV6) -> Result<(), CustomError> {
        println!("hola Peer a eliminar: {:?}", address);
        let mut node_state = self.node_state_ref.lock()?;
        node_state.remove_peer(address);
        println!("hola Borrado");
        Ok(())
    }

    fn handle_make_transaction(
        &mut self,
        outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        let transaction = match node_state.make_transaction(outputs, fee) {
            Ok(transaction) => transaction,
            Err(error) => {
                send_log(&self.logger_sender, Log::Error(error));
                return Ok(());
            }
        };
        drop(node_state);

        self.broadcast(transaction.clone())?;

        send_log(
            &self.logger_sender,
            Log::Message("Transaction broadcasted!".to_string()),
        );

        let mut node_state = self.node_state_ref.lock()?;
        node_state.append_pending_tx(transaction)?;
        self.gui_sender.send(GUIEvents::TransactionSent)?;

        Ok(())
    }

    fn handle_get_data_error(&mut self, inventory: Vec<Inventory>) -> Result<(), CustomError> {
        send_log(
            &self.logger_sender,
            Log::Message("Error requesting data,trying with another peer...".to_string()),
        );

        self.peer_action_sender
            .send(PeerAction::GetData(inventory))?;
        Ok(())
    }

    fn handle_get_headers_error(&mut self) -> Result<(), CustomError> {
        let node_state = self.node_state_ref.lock()?;
        let last_header = node_state.get_last_header_hash();
        drop(node_state);

        send_log(
            &self.logger_sender,
            Log::Message("Error requesting headers,trying with another peer...".to_string()),
        );

        self.peer_action_sender
            .send(PeerAction::GetHeaders(last_header))?;
        Ok(())
    }

    fn handle_new_headers(&mut self, new_headers: Headers) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        node_state.append_headers(&new_headers)?;
        drop(node_state);

        let headers_after_timestamp = &new_headers
            .headers
            .iter()
            .filter(|header| header.timestamp > START_DATE_IBD)
            .collect::<Vec<_>>();
        let chunks: Vec<&[&BlockHeader]> = headers_after_timestamp.chunks(5).collect();
        for chunk in chunks {
            self.request_block(chunk)?;
        }

        Ok(())
    }

    fn request_block(&mut self, headers: &[&BlockHeader]) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;

        let mut inventories = vec![];
        for header in headers {
            node_state.append_pending_block(header.hash())?;
            inventories.push(Inventory::new(InventoryType::Block, header.hash()));
        }

        drop(node_state);

        self.peer_action_sender
            .send(PeerAction::GetData(inventories))?;
        Ok(())
    }

    fn handle_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        if !node_state.is_block_pending(&block_hash)? {
            drop(node_state);
            return Ok(());
        }

        send_log(
            &self.logger_sender,
            Log::Message("New block received".to_string()),
        );

        node_state.append_block(block_hash, &block)?;
        let is_synced = node_state.is_synced();
        drop(node_state);

        if is_synced {
            self.broadcast_new_header(block.header)?;
        }
        Ok(())
    }

    fn handle_pending_transaction(&mut self, transaction: Transaction) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        if !node_state.is_synced() {
            drop(node_state);
            return Ok(());
        }

        let is_pending_new = node_state.append_pending_tx(transaction.clone())?;
        drop(node_state);

        if is_pending_new {
            self.broadcast(transaction)?;
        }
        Ok(())
    }

    fn handle_send_headers(&mut self, address: SocketAddrV6) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        node_state.peer_send_headers(address);
        Ok(())
    }

    fn handle_get_headers(
        &mut self,
        address: SocketAddrV6,
        getheaders: GetHeaders,
    ) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        node_state.peer_requested_headers(address);
        let headers = node_state.get_headers(getheaders);

        let message = Headers { headers };
        send_message(&mut node_state, address, message)
    }

    fn handle_get_data(
        &mut self,
        address: SocketAddrV6,
        getdata: GetData,
    ) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        for inventory in getdata.get_inventories() {
            match inventory.inventory_type {
                InventoryType::Block => {
                    match node_state.get_block(hash_as_string(inventory.hash.clone())) {
                        Ok(block) => send_message(&mut node_state, address, block)?,
                        Err(_) => {
                            let not_found = NotFound::new(vec![inventory.clone()]);
                            send_message(&mut node_state, address, not_found)?;
                        }
                    }
                }
                InventoryType::Tx => {
                    match node_state.get_pending_tx(&inventory.hash) {
                        Some(tx) => send_message(&mut node_state, address, tx)?,
                        None => {
                            let not_found = NotFound::new(vec![inventory.clone()]);
                            send_message(&mut node_state, address, not_found)?;
                        }
                    };
                }
                _ => {
                    let not_found = NotFound::new(vec![inventory.clone()]);
                    send_message(&mut node_state, address, not_found)?;
                }
            }
        }
        Ok(())
    }

    fn broadcast(&mut self, message: impl Message) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;

        let peers = node_state.get_peers();
        let mut peers_to_remove = vec![];
        for peer in peers {
            if message.send(&mut peer.stream).is_err() {
                peers_to_remove.push(peer.address);
            }
        }

        for address in peers_to_remove {
            node_state.remove_peer(address);
            send_log(
                &self.logger_sender,
                Log::Message(format!(
                    "Error sending message {} to peer {}",
                    message.get_command(),
                    address,
                )),
            );
        }

        Ok(())
    }

    fn broadcast_new_header(&self, header: BlockHeader) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        let mut peers_to_remove = vec![];
        for peer in node_state.get_peers() {
            if !peer.requested_headers {
                continue;
            }
            let sent = if peer.send_headers {
                println!("mandando header a peers via send_header {}", peer.address);
                let headers_msg = Headers {
                    headers: vec![header.clone()],
                };
                headers_msg.send(&mut peer.stream)
            } else {
                println!("mandando header a peers via inv {}", peer.address);
                let inventory = Inventory::new(InventoryType::Block, header.hash());
                let inv_msg = Inv::new(vec![inventory]);
                inv_msg.send(&mut peer.stream)
            };
            if sent.is_err() {
                peers_to_remove.push(peer.address);
            }
        }

        for address in peers_to_remove {
            node_state.remove_peer(address);
        }
        Ok(())
    }
}

fn send_message(
    node_state: &mut std::sync::MutexGuard<'_, NodeState>,
    address: SocketAddrV6,
    message: impl Message,
) -> Result<(), CustomError> {
    let peer = node_state.get_peer(&address);
    Ok(if let Some(peer) = peer {
        if peer.send(message).is_err() {
            node_state.remove_peer(address);
        }
    })
}
