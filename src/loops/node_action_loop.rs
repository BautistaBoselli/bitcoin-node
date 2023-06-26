use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::{block::Block, headers::Headers, transaction::Transaction},
    node_state::NodeState,
    peer::{NodeAction, PeerAction},
    structs::{
        block_header::BlockHeader,
        inventory::{Inventory, InventoryType},
    },
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeActionLoop {
    node_action_receiver: mpsc::Receiver<NodeAction>,
    peer_action_sender: mpsc::Sender<PeerAction>,
    logger_sender: mpsc::Sender<Log>,
    node_state_ref: Arc<Mutex<NodeState>>,
    npeers: u8,
}

impl NodeActionLoop {
    pub fn start(
        node_action_receiver: mpsc::Receiver<NodeAction>,
        peer_action_sender: mpsc::Sender<PeerAction>,
        logger_sender: mpsc::Sender<Log>,
        node_state_ref: Arc<Mutex<NodeState>>,
        npeers: u8,
    ) {
        let mut node_thread = Self {
            node_action_receiver,
            peer_action_sender,
            logger_sender,
            node_state_ref,
            npeers,
        };
        node_thread.event_loop();
    }

    pub fn event_loop(&mut self) {
        while let Ok(message) = self.node_action_receiver.recv() {
            let response = match message {
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
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on NodeActionLoop: {error}")),
                );
            }
        }
    }

    fn handle_make_transaction(
        &mut self,
        outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<(), CustomError> {
        let mut node_state = self
            .node_state_ref
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;

        let transaction = match node_state.make_transaction(outputs, fee) {
            Ok(transaction) => transaction,
            Err(error) => {
                send_log(&self.logger_sender, Log::Error(error));
                return Ok(());
            }
        };

        for _i in 0..self.npeers {
            self.peer_action_sender
                .send(PeerAction::SendTransaction(transaction.clone()))?;
        }

        drop(node_state);

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
        if !node_state.is_synced() {
            drop(node_state);
            return Ok(());
        }

        // println!(
        //     "New pending transaction received: {:?}",
        //     transaction.clone().hash()
        // );

        send_log(
            &self.logger_sender,
            Log::Message("New pending transaction received".to_string()),
        );
        node_state.append_pending_tx(transaction)?;
        drop(node_state);
        Ok(())
    }
}
