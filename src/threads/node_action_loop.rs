use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    sync::mpsc,
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    message::Message,
    messages::{
        block::{self, Block, OutPoint},
        headers::{BlockHeader, Headers},
        inv::{Inventory, InventoryType},
    },
    node::open_new_file,
    peer::{NodeAction, PeerAction},
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeActionLoop {
    pub node_action_receiver: mpsc::Receiver<NodeAction>,
    pub headers_file: File,
    pub peer_action_sender: mpsc::Sender<PeerAction>,
    pub logger_sender: mpsc::Sender<String>,
    pub headers: Vec<BlockHeader>,
    pub utxo_set: HashMap<OutPoint, block::TransactionOutput>,
    pub headers_sync: bool,
    pub blocks_sync: bool,
    pub utxo_sync: bool,
}

impl NodeActionLoop {
    pub fn spawn(
        node_action_receiver: mpsc::Receiver<NodeAction>,
        headers_file: File,
        peer_action_sender: mpsc::Sender<PeerAction>,
        headers: Vec<BlockHeader>,
        logger_sender: mpsc::Sender<String>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut node_thread = Self {
                node_action_receiver,
                headers_file,
                peer_action_sender,
                logger_sender,
                headers,
                utxo_set: HashMap::new(),
                headers_sync: false,
                blocks_sync: false,
                utxo_sync: false,
            };
            node_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) -> Result<(), CustomError> {
        while let Ok(message) = self.node_action_receiver.recv() {
            match message {
                NodeAction::Block((block_hash, block)) => self.handle_block(block_hash, block)?,
                NodeAction::NewHeaders(new_headers) => self.handle_new_headers(new_headers)?,
                NodeAction::GetHeadersError => self.handle_get_headers_error()?,
                NodeAction::GetDataError(inventory) => self.handle_get_data_error(inventory)?,
            }
        }
        Ok(())
    }

    fn handle_get_data_error(&mut self, inventory: Vec<Inventory>) -> Result<(), CustomError> {
        self.logger_sender
            .send(format!("Error requesting data,trying with another peer..."))?;
        self.peer_action_sender
            .send(PeerAction::GetData(inventory))?;
        Ok(())
    }

    fn handle_get_headers_error(&mut self) -> Result<(), CustomError> {
        let last_header = self.headers.last().map(|header| header.hash());
        self.peer_action_sender
            .send(PeerAction::GetHeaders(last_header))?;
        Ok(())
    }

    fn handle_new_headers(&mut self, mut new_headers: Headers) -> Result<(), CustomError> {
        self.headers_file
            .write_all(&new_headers.serialize_headers())?;

        let new_headers_count = new_headers.headers.len();

        let headers_after_timestamp = new_headers
            .headers
            .iter()
            .filter(|header| header.timestamp > START_DATE_IBD)
            .collect::<Vec<_>>();
        let chunks: Vec<&[&BlockHeader]> = headers_after_timestamp.chunks(5).collect();
        for chunk in chunks {
            self.request_block(chunk)?;
        }
        self.headers.append(&mut new_headers.headers);

        self.logger_sender.send(format!(
            "There are {} headers, new {}",
            self.headers.len(),
            new_headers_count
        ))?;

        self.verify_headers_sync(new_headers_count)?;
        Ok(())
    }

    fn request_block(&mut self, headers: &[&BlockHeader]) -> Result<(), CustomError> {
        let inventory = headers
            .iter()
            .map(|header| Inventory::new(InventoryType::GetBlock, header.hash()))
            .collect();
        self.peer_action_sender
            .send(PeerAction::GetData(inventory))?;
        Ok(())
    }

    fn handle_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let mut filename = String::with_capacity(2 * block_hash.len());
        for byte in block_hash {
            filename.push_str(format!("{:02X}", byte).as_str());
        }
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename))?;
        block_file.write_all(&block.serialize())?;

        //self.logger_sender.send(format!("New block downloaded"))?;

        match self.utxo_sync {
            true => update_utxo_set(&mut self.utxo_set, block),
            false => self.verify_blocks_sync()?,
        }
        Ok(())
    }

    fn verify_headers_sync(&mut self, new_headers_count: usize) -> Result<(), CustomError> {
        if self.headers_sync {
            return Ok(());
        }

        self.headers_sync = new_headers_count < 2000;
        if self.headers_sync {
            self.logger_sender
                .send("headers sync completed".to_string())?;
            self.verify_blocks_sync()?;
        }
        Ok(())
    }

    fn verify_blocks_sync(&mut self) -> Result<(), CustomError> {
        if self.blocks_sync {
            return Ok(());
        }
        //el -1 es por el archivo del gitkeep
        let blocks_downloaded = match fs::read_dir("store/blocks") {
            Ok(dir) => dir.count() - 1,
            Err(_) => 0,
        };
        let mut blocks_should_be_downloaded = 0;
        for header in self.headers.iter().rev() {
            if header.timestamp < START_DATE_IBD {
                break;
            }
            blocks_should_be_downloaded += 1;
        }
        self.blocks_sync = self.headers_sync && blocks_downloaded == blocks_should_be_downloaded;

        if self.blocks_sync {
            self.logger_sender.send(format!("blocks sync completed"))?;
            self.generate_utxo()?;
        }
        Ok(())
    }

    fn generate_utxo(&mut self) -> Result<(), CustomError> {
        let mut blocks_after_timestamp = 0;
        for header in self.headers.iter().rev() {
            if header.timestamp < START_DATE_IBD {
                break;
            }
            blocks_after_timestamp += 1;
        }
        self.logger_sender
            .send("Beginning the generation of the utxo (0%)...".to_string())?;

        let mut i = 0;
        let mut percentage = 0;
        for header in self.headers.iter().rev().take(blocks_after_timestamp).rev() {
            if i > blocks_after_timestamp / 10 {
                percentage += 10;
                self.logger_sender.send(format!(
                    "The generation of utxo is ({}%) completed...",
                    percentage
                ))?;
                i = 0;
            }
            let block_hash = header.hash();
            let mut filename = String::with_capacity(2 * block_hash.len());
            for byte in block_hash {
                filename.push_str(format!("{:02X}", byte).as_str());
            }
            let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename))?;
            let mut block_buffer = Vec::new();
            block_file.read_to_end(&mut block_buffer)?;
            let block = Block::parse(block_buffer)?;
            update_utxo_set(&mut self.utxo_set, block);
            i += 1;
        }
        self.utxo_sync = true;
        self.logger_sender
            .send(format!("The generation of utxo is (100%) completed"))?;
        self.logger_sender
            .send("Utxo generation is finished".to_string())?;
        Ok(())
    }
}

fn update_utxo_set(utxo_set: &mut HashMap<OutPoint, block::TransactionOutput>, block: Block) {
    for tx in block.transactions.iter() {
        for tx_in in tx.inputs.iter() {
            utxo_set.remove(&tx_in.previous_output);
        }
        for (index, tx_out) in tx.outputs.iter().enumerate() {
            let out_point = OutPoint {
                hash: tx.hash().clone(),
                index: index as u32,
            };
            utxo_set.insert(out_point, tx_out.clone());
        }
    }
}
