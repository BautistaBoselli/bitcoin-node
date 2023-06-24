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

#[derive(Debug, PartialEq)]
pub struct UTXOValue {
    pub tx_out: TransactionOutput,
    pub block_hash: Vec<u8>,
}

#[derive(PartialEq)]
pub struct UTXO {
    pub tx_set: HashMap<OutPoint, UTXOValue>,
    sync: bool,
    path: String,
}

impl UTXO {
    pub fn new(path: String) -> Result<Self, CustomError> {
        Ok(Self {
            tx_set: HashMap::new(),
            sync: false,
            path,
        })
    }

    pub fn wallet_balance(&self, wallet: &Wallet) -> Result<u64, CustomError> {
        let mut balance = 0;
        let pubkey_hash = wallet.get_pubkey_hash()?;
        for (_, value) in self.tx_set.iter() {
            if value.tx_out.is_sent_to_key(&pubkey_hash)? {
                balance += value.tx_out.value
            }
        }
        Ok(balance)
    }

    /// Returns all the unspent transactions from a particular wallet
    pub fn generate_wallet_utxo(
        &self,
        wallet: &Wallet,
    ) -> Result<Vec<(OutPoint, TransactionOutput)>, CustomError> {
        let pubkey_hash = wallet.get_pubkey_hash()?;

        let mut active_wallet_utxo = vec![];
        for (out_point, value) in self.tx_set.iter() {
            if value.tx_out.is_sent_to_key(&pubkey_hash)? {
                active_wallet_utxo.push((out_point.clone(), value.tx_out.clone()));
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
            Log::Message("Utxo generation is finished".to_string()),
        );

        Ok(())
    }

    fn restore_utxo(&mut self) -> Result<u32, CustomError> {
        let mut file = open_new_file(self.path.clone(), false)?;

        let mut saved_utxo_buffer = vec![];
        file.read_to_end(&mut saved_utxo_buffer)?;

        match Self::parse(saved_utxo_buffer.clone()) {
            Ok((last_timestamp, tx_set)) => {
                println!("Utxo set is restored from the file");
            }
            Err(e) => println!("Error: {}", e),
        }

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

    fn serialize(&mut self, last_timestamp: u32) -> Vec<u8> {
        let mut buffer = vec![];
        buffer.extend(last_timestamp.to_le_bytes());
        buffer.extend((self.tx_set.len() as u64).to_le_bytes());

        for (out_point, value) in self.tx_set.iter() {
            buffer.extend(out_point.serialize());
            buffer.extend(value.tx_out.serialize());
            buffer.extend(value.block_hash.clone());
        }
        buffer
    }

    pub fn parse(buffer: Vec<u8>) -> Result<(u32, HashMap<OutPoint, UTXOValue>), CustomError> {
        let mut parser = BufferParser::new(buffer);

        let last_timestamp = parser.extract_u32()?;
        let tx_set_len = parser.extract_u64()? as usize;
        let mut tx_set: HashMap<OutPoint, UTXOValue> = HashMap::new();

        for _i in 0..tx_set_len {
            let out_point = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;

            let value = UTXOValue {
                tx_out: TransactionOutput::parse(&mut parser)?,
                block_hash: parser.extract_buffer(32)?.to_vec(),
            };
            tx_set.insert(out_point, value);
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
                let value = UTXOValue {
                    tx_out: tx_out.clone(),
                    block_hash: block.header.hash(),
                };
                self.tx_set.insert(out_point.clone(), value);
            }
        }

        if save {
            self.save(block.header.timestamp)?;
        }

        Ok(())
    }

    fn save(&mut self, last_timestamp: u32) -> Result<(), CustomError> {
        let buffer = self.serialize(last_timestamp);

        remove_file(self.path.clone())?;
        let mut file = open_new_file(String::from(self.path.clone()), false)?;

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

#[cfg(test)]
mod tests {

    use gtk::glib::{self, Priority};

    use crate::{logger::Logger, wallet::get_script_pubkey};

    use super::*;

    #[test]
    fn test_save_restore() {
        let mut utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();

        let key1 = OutPoint {
            hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
            index: 1,
        };
        let value1 = UTXOValue {
            tx_out: TransactionOutput {
                value: 100,
                script_pubkey: get_script_pubkey(String::from(
                    "mscatccDgq7azndWHFTzvEuZuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
        };
        let key2 = OutPoint {
            hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
            index: 2,
        };
        let value2 = UTXOValue {
            tx_out: TransactionOutput {
                value: 200,
                script_pubkey: get_script_pubkey(String::from(
                    "mscatccDgq7azndWHFTzvEuZuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
        };
        let key3 = OutPoint {
            hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
            index: 3,
        };
        let value3 = UTXOValue {
            tx_out: TransactionOutput {
                value: 300,
                script_pubkey: get_script_pubkey(String::from(
                    "badnpccEgq7azndWHFTzvFuFuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ],
        };
        utxo_set.tx_set.insert(key1, value1);
        utxo_set.tx_set.insert(key2, value2);
        utxo_set.tx_set.insert(key3, value3);

        assert_eq!(utxo_set.tx_set.len(), 3);

        let last_timestamp = 1687623163;
        utxo_set.save(last_timestamp).unwrap();

        let mut utxo_set2 = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        utxo_set2.restore_utxo().unwrap();

        assert_eq!(utxo_set2.tx_set.len(), 3);
        assert_eq!(utxo_set2.tx_set, utxo_set.tx_set);
    }

    #[test]
    fn starting_index_calculation() {
        let header1 = BlockHeader {
            version: 536870912,
            prev_block_hash: [
                37, 167, 68, 172, 119, 180, 173, 121, 130, 113, 230, 183, 81, 26, 52, 142, 31, 52,
                247, 233, 68, 123, 190, 78, 10, 195, 189, 99, 0, 0, 0, 0,
            ]
            .to_vec(),
            merkle_root: [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 248, 125, 43, 136, 39, 116, 186, 43,
                114, 204, 35, 144, 47, 194, 229, 44, 97, 83, 110, 112, 229, 230,
            ]
            .to_vec(),
            timestamp: 1,
            bits: 486604799,
            nonce: 409655068,
        };

        let header2 = BlockHeader {
            version: 536870912,
            prev_block_hash: [
                37, 167, 68, 172, 119, 180, 173, 121, 130, 113, 230, 183, 81, 26, 52, 142, 31, 52,
                247, 233, 68, 123, 190, 78, 10, 195, 189, 99, 0, 0, 0, 0,
            ]
            .to_vec(),
            merkle_root: [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 248, 125, 43, 136, 39, 116, 186, 43,
                114, 204, 35, 144, 47, 194, 229, 44, 97, 83, 110, 112, 229, 230,
            ]
            .to_vec(),
            timestamp: 3,
            bits: 486604799,
            nonce: 409655068,
        };

        assert_eq!(
            calculate_starting_index(&vec![header1.clone(), header2.clone()], 2),
            1
        );
        assert_eq!(
            calculate_starting_index(&vec![header2.clone(), header1.clone()], 2),
            2
        );
        assert_eq!(calculate_starting_index(&vec![header2, header1], 0), 0);
    }

    #[test]
    fn utxo_serialization_and_parsing() {
        let mut utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let key: OutPoint = OutPoint {
            hash: [
                252, 47, 239, 163, 175, 36, 146, 56, 212, 168, 146, 23, 101, 29, 205, 186, 7, 67,
                240, 23, 75, 32, 175, 14, 221, 106, 150, 247, 21, 243, 205, 109,
            ]
            .to_vec(),
            index: 0,
        };
        let value = UTXOValue {
            tx_out: TransactionOutput {
                value: 100,
                script_pubkey: get_script_pubkey(String::from(
                    "mscatccDgq7azndWHFTzvEuZuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: [
                127, 47, 239, 163, 175, 36, 146, 56, 212, 168, 146, 23, 101, 29, 205, 186, 7, 67,
                240, 23, 75, 32, 175, 14, 221, 106, 150, 247, 21, 243, 205, 109,
            ]
            .to_vec(),
        };
        utxo_set.tx_set.insert(key, value);

        let buffer = utxo_set.serialize(100000);
        let (last_timestamp, parsed_tx_set) = UTXO::parse(buffer).unwrap();
        assert_eq!(last_timestamp, 100000);
        assert_eq!(utxo_set.tx_set, parsed_tx_set);
    }

    #[test]
    fn utxo_generation() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());

        let logger = match Logger::new(&String::new(), gui_sender) {
            Ok(logger) => logger,
            Err(error) => {
                println!("ERROR: {}", error);
                return;
            }
        };

        let logger_sender = logger.get_sender();
        let mut utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let header = BlockHeader {
            version: 536870912,
            prev_block_hash: [
                37, 167, 68, 172, 119, 180, 173, 121, 130, 113, 230, 183, 81, 26, 52, 142, 31, 52,
                247, 233, 68, 123, 190, 78, 10, 195, 189, 99, 0, 0, 0, 0,
            ]
            .to_vec(),
            merkle_root: [
                220, 9, 210, 68, 121, 44, 33, 165, 243, 235, 248, 125, 43, 136, 39, 116, 186, 43,
                114, 204, 35, 144, 47, 194, 229, 44, 97, 83, 110, 112, 229, 230,
            ]
            .to_vec(),
            timestamp: 1572523925,
            bits: 486604799,
            nonce: 409655068,
        };
        let headers = vec![header.clone()];
        utxo_set
            .generate(&headers, &mut logger_sender.clone())
            .unwrap();
        let block = Block::restore(header.hash_as_string()).unwrap();
        let mut utxo_len = 0;
        for tx in block.transactions {
            for tx_out in tx.outputs {
                if utxo_set.tx_set.values().any(|value| value.tx_out == tx_out) {
                    utxo_len += 1;
                }
            }
        }
        assert_eq!(utxo_set.tx_set.len(), utxo_len);
        assert_eq!(utxo_set.is_synced(), true);
    }

    #[test]
    fn wallet_utxo_generation() {
        let mut utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test_wallet"),
            String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"),
            String::from("privkey"),
            &utxo_set,
        )
        .unwrap();

        let key1 = OutPoint {
            hash: vec![],
            index: 1,
        };
        let tx_out1 = TransactionOutput {
            value: 100,
            script_pubkey: get_script_pubkey(String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"))
                .unwrap(),
        };
        let value1 = UTXOValue {
            tx_out: tx_out1.clone(),
            block_hash: vec![],
        };
        let key2 = OutPoint {
            hash: vec![],
            index: 2,
        };
        let tx_out2 = TransactionOutput {
            value: 200,
            script_pubkey: get_script_pubkey(String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"))
                .unwrap(),
        };
        let value2 = UTXOValue {
            tx_out: tx_out2.clone(),
            block_hash: vec![],
        };
        let tx_out3 = TransactionOutput {
            value: 100,
            script_pubkey: get_script_pubkey(String::from("badnpccDgq7azndWHFTzvFuZuywCsUvTRu"))
                .unwrap(),
        };

        let key3 = OutPoint {
            hash: vec![],
            index: 3,
        };
        let value3 = UTXOValue {
            tx_out: tx_out3.clone(),

            block_hash: vec![],
        };
        utxo_set.tx_set.insert(key1.clone(), value1);
        utxo_set.tx_set.insert(key2.clone(), value2);
        utxo_set.tx_set.insert(key3.clone(), value3);
        assert!(utxo_set.generate_wallet_utxo(&wallet).unwrap().len() == 2);
        assert!(utxo_set
            .generate_wallet_utxo(&wallet)
            .unwrap()
            .contains(&(key1, tx_out1)));
        assert!(utxo_set
            .generate_wallet_utxo(&wallet)
            .unwrap()
            .contains(&(key2, tx_out2)));
    }

    #[test]
    fn correct_wallet_balance() {
        let mut utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test_wallet"),
            String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"),
            String::from("privkey"),
            &utxo_set,
        )
        .unwrap();

        let key1 = OutPoint {
            hash: vec![],
            index: 1,
        };
        let value1 = UTXOValue {
            tx_out: TransactionOutput {
                value: 100,
                script_pubkey: get_script_pubkey(String::from(
                    "mscatccDgq7azndWHFTzvEuZuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![],
        };
        let key2 = OutPoint {
            hash: vec![],
            index: 2,
        };
        let value2 = UTXOValue {
            tx_out: TransactionOutput {
                value: 200,
                script_pubkey: get_script_pubkey(String::from(
                    "mscatccDgq7azndWHFTzvEuZuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![],
        };
        let key3 = OutPoint {
            hash: vec![],
            index: 3,
        };
        let value3 = UTXOValue {
            tx_out: TransactionOutput {
                value: 300,
                script_pubkey: get_script_pubkey(String::from(
                    "badnpccEgq7azndWHFTzvFuFuywCsUvTRu",
                ))
                .unwrap(),
            },
            block_hash: vec![],
        };
        utxo_set.tx_set.insert(key1, value1);
        utxo_set.tx_set.insert(key2, value2);
        utxo_set.tx_set.insert(key3, value3);
        assert_eq!(utxo_set.wallet_balance(&wallet).unwrap(), 300);
    }
}
