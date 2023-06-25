use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    message::Message,
    messages::{get_data::GetData, transaction::Transaction},
    peer::{request_headers, NodeAction, PeerAction},
    structs::inventory::Inventory,
};

pub struct PeerActionLoop {
    pub peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    pub version: i32,
    pub stream: TcpStream,
    pub logger_sender: mpsc::Sender<Log>,
    pub node_action_sender: mpsc::Sender<NodeAction>,
}

impl PeerActionLoop {
    pub fn spawn(
        peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        version: i32,
        stream: TcpStream,
        logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut peer_action_thread = Self {
                peer_action_receiver,
                version,
                stream,
                logger_sender,
                node_action_sender,
            };
            peer_action_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) -> Result<(), CustomError> {
        loop {
            let peer_message = self
                .peer_action_receiver
                .lock()
                .map_err(|_| CustomError::CannotLockGuard)?
                .recv()?;
            let response = match peer_message {
                PeerAction::GetHeaders(last_header) => self.handle_getheaders(last_header),
                PeerAction::GetData(inventories) => self.handle_getdata(inventories),
                PeerAction::SendTransaction(transaction) => {
                    self.handle_send_transaction(&transaction)
                }
                PeerAction::Terminate => {
                    break;
                }
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on PeerActionLoop: {error}")),
                );
            }
        }
        Ok(())
    }

    fn handle_send_transaction(&mut self, transaction: &Transaction) -> Result<(), CustomError> {
        transaction.send(&mut self.stream)?;
        send_log(
            &self.logger_sender,
            Log::Message("Sending transaction".to_string()),
        );
        Ok(())
    }
    fn handle_getdata(&mut self, inventories: Vec<Inventory>) -> Result<(), CustomError> {
        let inventories_clone = inventories.clone();
        let request = GetData::new(inventories).send(&mut self.stream);
        if request.is_err() {
            self.node_action_sender
                .send(NodeAction::GetDataError(inventories_clone))?;
        };
        Ok(())
    }

    fn handle_getheaders(&mut self, last_header: Option<Vec<u8>>) -> Result<(), CustomError> {
        request_headers(
            last_header,
            self.version,
            &mut self.stream,
            &self.logger_sender,
            &self.node_action_sender,
        )
    }
}
