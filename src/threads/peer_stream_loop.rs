use std::{
    io::Read,
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    message::{Message, MessageHeader},
    messages::{
        block::Block,
        get_data::GetData,
        headers::Headers,
        inv::{Inv, Inventory, InventoryType},
        ping_pong::{Ping, Pong},
        transaction::Transaction,
    },
    node_state::NodeState,
    peer::{request_headers, NodeAction},
};

pub struct PeerStreamLoop {
    pub stream: TcpStream,
    pub node_action_sender: mpsc::Sender<NodeAction>,
    pub version: i32,
    pub logger_sender: mpsc::Sender<Log>,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl PeerStreamLoop {
    pub fn spawn(
        version: i32,
        stream: TcpStream,
        logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
        node_state_ref: Arc<Mutex<NodeState>>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut peer_action_thread = Self {
                version,
                stream,
                logger_sender,
                node_action_sender,
                node_state_ref,
            };
            peer_action_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) -> Result<(), CustomError> {
        loop {
            let response_header = MessageHeader::read(&mut self.stream)?;

            let response = match response_header.command.as_str() {
                "headers" => self.handle_headers(&response_header),
                "block" => self.handle_block(&response_header),
                "ping" => self.handle_ping(&response_header),
                "inv" => self.handle_inv(&response_header),
                "tx" => self.handle_tx(&response_header),
                "notfound" => self.handle_notfound(&response_header),
                "getdata" => self.handle_transaction_request(&response_header),
                _ => self.ignore_message(&response_header),
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on PeerStreamLoop: {}", error)),
                );
            }
        }
    }

    fn handle_transaction_request(
        &mut self,
        response_header: &MessageHeader,
    ) -> Result<(), CustomError> {
        let get_data = GetData::read(&mut self.stream, response_header.payload_size)?;
        let mut node_state_ref = self.node_state_ref.lock()?;
        let transaction =
            node_state_ref.get_transaction_to_send(get_data.get_inventories().clone());
        if let Some(tx) = transaction {
            tx.send(&mut self.stream)?;
            println!("enviamos la transaccion");
        }
        Ok(())
    }

    fn handle_headers(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let response = match Headers::read(&mut self.stream, response_header.payload_size) {
            Ok(response) => response,
            Err(_) => {
                self.node_action_sender.send(NodeAction::GetHeadersError)?;
                return Ok(());
            }
        };

        if response.headers.len() == 2000 {
            let last_header = response.headers.last().map(|h| h.hash());
            request_headers(
                last_header,
                self.version,
                &mut self.stream,
                &self.logger_sender,
                &self.node_action_sender,
            )?;
        }
        self.node_action_sender
            .send(NodeAction::NewHeaders(response))?;
        Ok(())
    }

    fn handle_block(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let block = Block::read(&mut self.stream, response_header.payload_size)?;
        match block.create_merkle_root() {
            Ok(_) => {
                self.node_action_sender
                    .send(NodeAction::Block((block.header.hash(), block)))?;
            }
            Err(_) => {
                let inventory = Inventory::new(InventoryType::Block, block.header.hash());

                self.node_action_sender
                    .send(NodeAction::GetDataError(vec![inventory]))?;

                send_log(
                    &self.logger_sender,
                    Log::Message(format!(
                        "Error validating the merkle root in the block: {:?}",
                        block.header.hash()
                    )),
                );
            }
        };
        Ok(())
    }

    fn handle_ping(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let ping = Ping::read(&mut self.stream, response_header.payload_size)?;
        let pong = Pong { nonce: ping.nonce };
        pong.send(&mut self.stream)?;
        Ok(())
    }

    fn handle_inv(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let inv = Inv::read(&mut self.stream, response_header.payload_size)?;
        for inventory in inv.inventories {
            if inventory.inventory_type == InventoryType::Tx {
                let message = GetData::new(vec![inventory]);
                message.send(&mut self.stream)?;
            }
        }
        Ok(())
    }

    fn handle_tx(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let tx = Transaction::read(&mut self.stream, response_header.payload_size)?;
        self.node_action_sender
            .send(NodeAction::PendingTransaction(tx))?;
        Ok(())
    }

    fn handle_notfound(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let notfound = GetData::read(&mut self.stream, response_header.payload_size)?;
        let inventories = notfound.get_inventories().to_owned();
        self.node_action_sender
            .send(NodeAction::GetDataError(inventories))?;
        Ok(())
    }

    fn ignore_message(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let cmd = response_header.command.as_str();
        if cmd != "alert" && cmd != "addr" && cmd != "sendheaders" {
            send_log(
                &self.logger_sender,
                Log::Message(format!(
                    "Received unknown command: {:?}",
                    response_header.command
                )),
            );
        }
        let mut buffer: Vec<u8> = vec![0; response_header.payload_size as usize];
        self.stream.read_exact(&mut buffer)?;
        Ok(())
    }
}
