use std::{
    io::Read,
    net::TcpStream,
    sync::mpsc,
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
        inv::Inv,
        ping_pong::{Ping, Pong},
        transaction::Transaction,
    },
    peer::{request_headers, NodeAction},
    structs::{
        block_header::BlockHeader,
        inventory::{Inventory, InventoryType},
    },
};

pub struct PeerStreamLoop {
    pub stream: TcpStream,
    pub node_action_sender: mpsc::Sender<NodeAction>,
    pub version: i32,
    pub logger_sender: mpsc::Sender<Log>,
}

impl PeerStreamLoop {
    #[must_use]
    pub fn spawn(
        version: i32,
        stream: TcpStream,
        logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut peer_action_thread = Self {
                stream,
                node_action_sender,
                version,
                logger_sender,
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
                _ => self.ignore_message(&response_header),
            };

            if let Err(error) = response {
                send_log(
                    &self.logger_sender,
                    Log::Message(format!("Error on PeerStreamLoop: {error}")),
                );
            }
        }
    }

    fn handle_headers(&mut self, response_header: &MessageHeader) -> Result<(), CustomError> {
        let response =
            if let Ok(response) = Headers::read(&mut self.stream, response_header.payload_size) {
                response
            } else {
                self.node_action_sender.send(NodeAction::GetHeadersError)?;
                return Ok(());
            };

        if response.headers.len() == 2000 {
            let last_header = response.headers.last().map(BlockHeader::hash);
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
        if block.create_merkle_root().is_ok() {
            self.node_action_sender
                .send(NodeAction::Block((block.header.hash(), block)))?;
        } else {
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
        let inventories = notfound.get_inventories().clone();
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
