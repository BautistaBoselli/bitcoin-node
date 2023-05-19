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
    peer::{PeerAction, PeerResponse},
};

const FECHA_INICIO_IBD: u32 = 1681095630;

pub struct NodeThread {
    pub peers_response_receiver: mpsc::Receiver<PeerResponse>,
    pub headers_file: File,
    pub peers_sender: mpsc::Sender<PeerAction>,
    pub logger_sender: mpsc::Sender<String>,
    pub headers: Vec<BlockHeader>,
    pub utxo_set: HashMap<OutPoint, block::TransactionOutput>,
    pub headers_sync: bool,
    pub blocks_sync: bool,
    pub utxo_sync: bool,
}

impl NodeThread {
    pub fn spawn(
        peers_response_receiver: mpsc::Receiver<PeerResponse>,
        headers_file: File,
        peers_sender: mpsc::Sender<PeerAction>,
        headers: Vec<BlockHeader>,
        logger_sender: mpsc::Sender<String>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut node_thread = Self {
                peers_response_receiver,
                headers_file,
                peers_sender,
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
        while let Ok(message) = self.peers_response_receiver.recv() {
            match message {
                PeerResponse::Block((block_hash, block)) => self.handle_block(block_hash, block)?,
                PeerResponse::NewHeaders(new_headers) => self.handle_new_headers(new_headers)?,
                PeerResponse::GetHeadersError => self.handle_get_headers_error()?,
                PeerResponse::GetDataError(inventory) => self.handle_get_data_error(inventory)?,
            }
        }
        Ok(())
    }

    fn handle_get_data_error(&mut self, inventory: Vec<Inventory>) -> Result<(), CustomError> {
        self.peers_sender.send(PeerAction::GetData(inventory))?;
        Ok(())
    }

    fn handle_get_headers_error(&mut self) -> Result<(), CustomError> {
        let last_header = self.headers.last().map(|header| header.hash());
        self.peers_sender
            .send(PeerAction::GetHeaders(last_header))?;
        Ok(())
    }

    fn handle_new_headers(&mut self, mut new_headers: Headers) -> Result<(), CustomError> {
        self.headers_file
            .write_all(&new_headers.serialize_headers())?;

        let new_headers_count = new_headers.headers.len();
        self.headers_sync = new_headers_count < 2000;
        self.verify_blocks_sync()?;

        new_headers
            .headers
            .iter()
            .filter(|header| header.timestamp > FECHA_INICIO_IBD)
            .collect::<Vec<_>>()
            .chunks(5)
            .for_each(|headers| {
                let inventory = headers
                    .iter()
                    .map(|header| Inventory::new(InventoryType::GetBlock, header.hash()))
                    .collect();
                self.peers_sender
                    .send(PeerAction::GetData(inventory))
                    .unwrap();
            });
        self.headers.append(&mut new_headers.headers);
        println!(
            "Hay {} headers,nuevos {}",
            self.headers.len(),
            new_headers_count
        );
        Ok(())
    }

    fn handle_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let mut filename = String::with_capacity(2 * block_hash.len());
        for byte in block_hash {
            filename.push_str(format!("{:02X}", byte).as_str());
        }
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename))?;
        block_file.write_all(&block.serialize())?;

        if !self.blocks_sync {
            self.verify_blocks_sync()?;
        }

        println!("hola");
        println!("Hay {} elementos en el utxo_set 1/2", self.utxo_set.len());
        if self.utxo_sync {
            update_utxo_set(&mut self.utxo_set, block);
        }
        println!("Hay {} elementos en el utxo_set 2/2", self.utxo_set.len());
        Ok(())
    }

    fn verify_blocks_sync(&mut self) -> Result<(), CustomError> {
        //el -1 es por el archivo del gitkeep
        let blocks_downloaded = fs::read_dir("store/blocks").unwrap().into_iter().count() - 1;
        let mut blocks_should_be_downloaded = 0;
        for header in self.headers.iter().rev() {
            if header.timestamp < FECHA_INICIO_IBD {
                break;
            }
            blocks_should_be_downloaded += 1;
        }
        self.blocks_sync = self.headers_sync && blocks_downloaded == blocks_should_be_downloaded;
        println!("blocks_sync: {}", self.blocks_sync);
        println!("headers_sync: {}", self.headers_sync);
        println!(
            "blocks_downloaded: {}, blocks_should_be_downloaded: {}",
            blocks_downloaded, blocks_should_be_downloaded
        );

        if self.blocks_sync {
            self.logger_sender
                .send(format!("sincronizacion de bloques finalizada"))?;
            self.generate_utxo().unwrap();
        }
        Ok(())
    }

    fn generate_utxo(&mut self) -> Result<(), CustomError> {
        let mut blocks_after_timestamp = 0;
        for header in self.headers.iter().rev() {
            if header.timestamp < FECHA_INICIO_IBD {
                break;
            }
            blocks_after_timestamp += 1;
        }
        println!("hay que recorrer {} bloques", blocks_after_timestamp);
        for header in self.headers.iter().rev().take(blocks_after_timestamp).rev() {
            //println!("recorriendo el bloque con hash {:?}", header.hash());
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
        }
        self.utxo_sync = true;
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
