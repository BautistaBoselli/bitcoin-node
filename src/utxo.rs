use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::{
        block::Block,
        headers::BlockHeader,
        transaction::{OutPoint, TransactionOutput},
    },
    node_state::open_new_file,
    parser::BufferParser,
    wallet::Wallet,
};
use std::{
    collections::HashMap,
    fs::remove_file,
    io::{Read, Write},
    sync::mpsc::Sender,
    vec,
};

const START_DATE_IBD: u32 = 1681095630;

pub struct UTXO {
    pub tx_set: HashMap<OutPoint, TransactionOutput>,
    sync: bool,
}

impl UTXO {
    pub fn new() -> Result<Self, CustomError> {
        Ok(Self {
            tx_set: HashMap::new(),
            sync: false,
        })
    }

    pub fn wallet_balance(&self, wallet: &Wallet) -> Result<u64, CustomError> {
        let mut balance = 0;
        let pubkey_hash = wallet.get_pubkey_hash()?;
        for (_, tx_out) in self.tx_set.iter() {
            if tx_out.is_sent_to_key(&pubkey_hash) {
                balance += tx_out.value;
            }
        }
        Ok(balance)
    }

    pub fn wallet_utxo(
        &self,
        wallet: &Wallet,
    ) -> Result<Vec<(OutPoint, TransactionOutput)>, CustomError> {
        let pubkey_hash = wallet.get_pubkey_hash()?;

        let mut active_wallet_utxo = vec![];
        for (out_point, tx_out) in self.tx_set.iter() {
            if tx_out.is_sent_to_key(&pubkey_hash) {
                active_wallet_utxo.push((out_point.clone(), tx_out.clone()));
            }
        }

        Ok(active_wallet_utxo)
    }

    pub fn is_synced(&self) -> bool {
        self.sync
    }

    pub fn generate(
        &mut self,
        headers: &Vec<BlockHeader>,
        logger_sender: &mut Sender<Log>,
    ) -> Result<(), CustomError> {
        let mut last_timestamp = self.restore_utxo()?;
        let starting_index = calculate_starting_index(headers, last_timestamp);
        send_log(
            logger_sender,
            Log::Message(format!(
                "Utxo generation is starting ({} new blocks)",
                headers.len() - starting_index
            )),
        );
        self.update_from_headers(headers, starting_index, logger_sender, &mut last_timestamp)?;

        self.sync = true;
        self.save(last_timestamp)?;

        send_log(
            logger_sender,
            Log::Message("Utxo generation is (100%) completed...".to_string()),
        );
        send_log(
            logger_sender,
            Log::Message("Utxo generation is finished...".to_string()),
        );

        Ok(())
    }

    fn restore_utxo(&mut self) -> Result<u32, CustomError> {
        let mut file = open_new_file(String::from("store/utxo.bin"), false)?;

        let mut saved_utxo_buffer = vec![];
        file.read_to_end(&mut saved_utxo_buffer)?;
        let (last_timestamp, tx_set) = match Self::parse(saved_utxo_buffer) {
            Ok((last_timestamp, tx_set)) => (last_timestamp, tx_set),
            Err(_) => (START_DATE_IBD, HashMap::new()),
        };

        self.tx_set = tx_set;
        Ok(last_timestamp)
    }

    fn update_from_headers(
        &mut self,
        headers: &Vec<BlockHeader>,
        starting_index: usize,
        logger_sender: &mut Sender<Log>,
        last_timestamp: &mut u32,
    ) -> Result<(), CustomError> {
        let mut i = 0;
        let mut percentage = 0;

        for (_index, header) in headers.iter().enumerate().skip(starting_index) {
            if i > (headers.len() - starting_index) / 10 {
                percentage += 10;
                send_log(
                    logger_sender,
                    Log::Message(format!("Utxo generation is ({}%) completed...", percentage)),
                );
                i = 0;
            }
            let block = Block::restore(header.hash_as_string())?;
            self.update_from_block(&block, false)?;
            *last_timestamp = header.timestamp;
            i += 1;
        }
        Ok(())
    }

    pub fn parse(
        buffer: Vec<u8>,
    ) -> Result<(u32, HashMap<OutPoint, TransactionOutput>), CustomError> {
        let mut parser = BufferParser::new(buffer);

        let last_timestamp = parser.extract_u32()?;
        let tx_set_len = parser.extract_u64()? as usize;
        let mut tx_set: HashMap<OutPoint, TransactionOutput> = HashMap::new();

        for _i in 0..tx_set_len {
            let out_point = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;
            let tx = TransactionOutput::parse(&mut parser)?;
            tx_set.insert(out_point, tx);
        }

        Ok((last_timestamp, tx_set))
    }

    pub fn update_from_block(&mut self, block: &Block, save: bool) -> Result<(), CustomError> {
        for tx in &block.transactions {
            for tx_in in &tx.inputs {
                self.tx_set.remove(&tx_in.previous_output);
            }
            for (index, tx_out) in tx.outputs.iter().enumerate() {
                let out_point = OutPoint {
                    hash: tx.hash().clone(),
                    index: index as u32,
                };
                self.tx_set.insert(out_point.clone(), tx_out.clone());
            }
        }

        if save {
            self.save(block.header.timestamp)?;
        }

        Ok(())
    }

    fn save(&mut self, last_timestamp: u32) -> Result<(), CustomError> {
        let mut buffer = vec![];
        buffer.extend(last_timestamp.to_le_bytes());
        buffer.extend((self.tx_set.len() as u64).to_le_bytes());

        for (out_point, tx_out) in self.tx_set.iter() {
            buffer.extend(out_point.serialize());
            buffer.extend(tx_out.serialize());
        }

        remove_file("store/utxo.bin")?;
        let mut file = open_new_file(String::from("store/utxo.bin"), false)?;

        file.write_all(&buffer)?;
        Ok(())
    }
}

fn calculate_starting_index(headers: &Vec<BlockHeader>, last_timestamp: u32) -> usize {
    let new_headers_len = headers
        .iter()
        .rev()
        .position(|header| header.timestamp <= last_timestamp);

    match new_headers_len {
        Some(new_headers_len) => headers.len() - new_headers_len,
        None => 0,
    }
}
