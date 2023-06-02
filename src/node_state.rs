use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    sync::mpsc,
};

use crate::{
    error::CustomError,
    message::Message,
    messages::{
        block::{self, Block, OutPoint},
        headers::{hash_as_string, BlockHeader, Headers},
    },
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeState {
    logger_sender: mpsc::Sender<String>,
    headers_file: File,
    headers: Vec<BlockHeader>,
    utxo_set: HashMap<OutPoint, block::TransactionOutput>,
    headers_sync: bool,
    blocks_sync: bool,
    utxo_sync: bool,
}

impl NodeState {
    pub fn new(logger_sender: mpsc::Sender<String>) -> Result<Self, CustomError> {
        let mut headers_file = open_new_file(String::from("store/headers.bin"))?;

        let mut saved_headers_buffer = vec![];
        headers_file.read_to_end(&mut saved_headers_buffer)?;

        let headers = match Headers::parse_headers(saved_headers_buffer) {
            Ok(headers) => headers,
            Err(_) => vec![],
        };

        Ok(Self {
            logger_sender,
            headers_file,
            headers,
            utxo_set: HashMap::new(),
            headers_sync: false,
            blocks_sync: false,
            utxo_sync: false,
        })
    }

    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.last().map(|header| header.hash())
    }

    pub fn append_headers(&mut self, headers: &mut Headers) -> Result<(), CustomError> {
        self.headers_file.write_all(&headers.serialize_headers())?;
        let headers_count = headers.headers.len();

        self.headers.append(&mut headers.headers);

        self.logger_sender.send(format!(
            "There are {} headers, new {}",
            self.headers.len(),
            headers_count
        ))?;

        self.verify_headers_sync(headers_count)
    }

    pub fn append_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let filename = hash_as_string(block_hash);
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename))?;
        block_file.write_all(&block.serialize())?;

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
            self.logger_sender
                .send("blocks sync completed".to_string())?;
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
            let hash = header.hash_as_string();
            let mut block_file = open_new_file(format!("store/blocks/{}.bin", hash))?;
            let mut block_buffer = Vec::new();
            block_file.read_to_end(&mut block_buffer)?;
            let block = Block::parse(block_buffer)?;
            update_utxo_set(&mut self.utxo_set, block);
            i += 1;
        }
        self.utxo_sync = true;
        self.logger_sender
            .send("The generation of utxo is (100%) completed".to_string())?;
        self.logger_sender
            .send("Utxo generation is finished".to_string())?;
        Ok(())
    }

    pub fn is_headers_sync(&self) -> bool {
        self.headers_sync
    }

    pub fn is_blocks_sync(&self) -> bool {
        self.blocks_sync
    }

    pub fn is_utxo_sync(&self) -> bool {
        self.utxo_sync
    }

    pub fn number_of_headers(&self) -> usize {
        self.headers.len()
    }
}

pub fn open_new_file(path_to_file: String) -> Result<std::fs::File, CustomError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true)
        .open(path_to_file)?;
    Ok(file)
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
