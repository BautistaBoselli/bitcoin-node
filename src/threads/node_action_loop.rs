use std::{
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use gtk::glib;

use crate::{
    error::CustomError,
    gui::init::GUIActions,
    logger::{send_log, Log},
    messages::{
        block::Block,
        headers::{BlockHeader, Headers},
        inv::{Inventory, InventoryType},
        transaction::Transaction,
    },
    node_state::NodeState,
    peer::{NodeAction, PeerAction},
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeActionLoop {
    pub node_action_receiver: mpsc::Receiver<NodeAction>,
    pub peer_action_sender: mpsc::Sender<PeerAction>,
    pub logger_sender: mpsc::Sender<Log>,
    pub gui_sender: glib::Sender<GUIActions>,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl NodeActionLoop {
    pub fn spawn(
        node_action_receiver: mpsc::Receiver<NodeAction>,
        peer_action_sender: mpsc::Sender<PeerAction>,
        logger_sender: mpsc::Sender<Log>,
        gui_sender: glib::Sender<GUIActions>,
        node_state_ref: Arc<Mutex<NodeState>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let mut node_thread = Self {
                node_action_receiver,
                peer_action_sender,
                logger_sender,
                gui_sender,
                node_state_ref,
            };
            node_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) {
        while let Ok(message) = self.node_action_receiver.recv() {
            let response = match message {
                NodeAction::Block((block_hash, block)) => self.handle_block(block_hash, block),
                NodeAction::NewHeaders(new_headers) => self.handle_new_headers(new_headers),
                NodeAction::GetHeadersError => self.handle_get_headers_error(),
                NodeAction::GetDataError(inventory) => self.handle_get_data_error(inventory),
                NodeAction::PendingTransaction(transaction) => {
                    self.handle_pending_transaction(transaction)
                }
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on NodeActionLoop: {}", error)),
                );
            }
        }
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

    fn handle_new_headers(&mut self, mut new_headers: Headers) -> Result<(), CustomError> {
        let headers_after_timestamp = new_headers
            .headers
            .iter()
            .filter(|header| header.timestamp > START_DATE_IBD)
            .collect::<Vec<_>>();
        let chunks: Vec<&[&BlockHeader]> = headers_after_timestamp.chunks(5).collect();
        for chunk in chunks {
            self.request_block(chunk)?;
        }

        let mut node_state = self.node_state_ref.lock()?;
        node_state.append_headers(&mut new_headers)?;

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

        node_state.append_block(block_hash, block)?;
        drop(node_state);

        Ok(())
    }

    fn handle_pending_transaction(&mut self, transaction: Transaction) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        println!("Transaction pendiente: {:?}", transaction);
        node_state.append_pending_transaction(transaction)?;
        drop(node_state);
        Ok(())
    }
}
