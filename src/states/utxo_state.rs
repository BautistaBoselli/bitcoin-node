use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::block::Block,
    parser::BufferParser,
    structs::tx_output::TransactionOutput,
    structs::{block_header::BlockHeader, outpoint::OutPoint},
    utils::{calculate_index_from_timestamp, open_new_file},
    wallet::Wallet,
};
use std::{
    collections::HashMap,
    fs::remove_file,
    io::{Read, Write},
    path::Path,
    process::exit,
    sync::mpsc::Sender,
    vec,
};

pub const START_DATE_IBD: u32 = 1681095630;

#[derive(Debug, PartialEq, Clone)]
/// UTXOValue es una estructura que contiene los valores que necesitamos guardar de las UTXO.
/// Los elementos son:
/// - tx_out: TransactionOutput.
/// - block_hash: Hash del bloque donde se encuentra el UTXO.
/// - block_timestamp: Timestamp del bloque donde se encuentra el UTXO.
pub struct UTXOValue {
    pub tx_out: TransactionOutput,
    pub block_hash: Vec<u8>,
    pub block_timestamp: u32,
}

#[derive(PartialEq)]
/// UTXO es una estructura que contiene los elementos necesarios para manejar las UTXO.
/// Los elementos son:
/// - tx_set: HashMap que contiene las UTXO con su OutPoint y UTXOValue.
/// - sync: Indica si las UTXO estan sincronizadas con la red.
/// - path: Path del archivo donde se guardan las UTXO.
///
/// El UTXO tiene un sistema de guardado tipo checkpoint
/// donde cada vez que se actualiza genera un archivo donde lista los utxo del momento y el timestamp del ultimo bloque procesado.
pub struct UTXO {
    pub tx_set: HashMap<OutPoint, UTXOValue>,
    sync: bool,
    store_path: String,
    path: String,
}

impl UTXO {
    /// Inicializa las UTXO con el path del archivo donde se almacena.
    /// El utxo comienza desincronizado y vacio.
    pub fn new(store_path: String, path: String) -> Result<Self, CustomError> {
        Ok(Self {
            tx_set: HashMap::new(),
            sync: false,
            store_path,
            path,
        })
    }

    /// Devuelve el balance de una wallet.
    pub fn wallet_balance(&self, wallet: &Wallet) -> Result<u64, CustomError> {
        let mut balance = 0;
        let pubkey_hash = wallet.get_pubkey_hash()?;
        for value in self.tx_set.values() {
            if value.tx_out.is_sent_to_key(&pubkey_hash)? {
                balance += value.tx_out.value;
            }
        }
        Ok(balance)
    }

    /// Devuelve las UTXO de una wallet.
    pub fn generate_wallet_utxo(
        &self,
        wallet: &Wallet,
    ) -> Result<Vec<(OutPoint, UTXOValue)>, CustomError> {
        let pubkey_hash = wallet.get_pubkey_hash()?;

        let mut active_wallet_utxo = vec![];
        for (out_point, value) in &self.tx_set {
            if value.tx_out.is_sent_to_key(&pubkey_hash)? {
                active_wallet_utxo.push((out_point.clone(), value.clone()));
            }
        }

        Ok(active_wallet_utxo)
    }

    /// Devuelve si el utxo esta sincronizado.
    pub fn is_synced(&self) -> bool {
        self.sync
    }

    /// Genera las UTXO a partir de los headers.
    /// Si el archivo donde se guardan las UTXO no existe, se crea.
    /// Si el archivo existe, se restauran las UTXO hasta ese punto y se recorren unicamente los bloques posteriores al timestamp guardado en el archivo.
    pub fn generate(
        &mut self,
        headers: &Vec<BlockHeader>,
        logger_sender: &mut Sender<Log>,
    ) -> Result<(), CustomError> {
        let last_block_hash = self.restore_utxo()?.unwrap_or_else(|| {
            let first_block_index = calculate_index_from_timestamp(headers, START_DATE_IBD);
            headers[first_block_index].hash().clone()
        });

        let new_last_block_hash =
            self.update_from_headers(headers, last_block_hash, logger_sender)?;

        self.sync = true;
        self.save(new_last_block_hash)?;

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

    fn restore_utxo(&mut self) -> Result<Option<Vec<u8>>, CustomError> {
        let path = format!("{}/{}", self.store_path, self.path);
        let mut file = open_new_file(path, false)?;

        let mut saved_utxo_buffer = vec![];
        file.read_to_end(&mut saved_utxo_buffer)?;

        let (last_block_hash, tx_set) = match Self::parse(saved_utxo_buffer) {
            Ok((last_block_hash, tx_set)) => (Some(last_block_hash), tx_set),
            Err(_) => (None, HashMap::new()),
        };

        self.tx_set = tx_set;
        Ok(last_block_hash)
    }

    fn update_from_headers(
        &mut self,
        headers: &Vec<BlockHeader>,
        last_block_hash: Vec<u8>,
        logger_sender: &mut Sender<Log>,
    ) -> Result<Vec<u8>, CustomError> {
        let mut i = 0;
        let mut percentage = 0;

        let mut last_block_hash = last_block_hash;

        let block_position = headers
            .iter()
            .rev()
            .position(|h| *h.hash() == last_block_hash);

        println!("block_position: {:?}", block_position);
        println!("blocks len: {:?}", headers.len());

        let starting_index = match block_position {
            Some(position) => headers.len() - position,
            None => calculate_index_from_timestamp(headers, START_DATE_IBD),
        };

        send_log(
            logger_sender,
            Log::Message(format!(
                "Utxo generation is starting ({} new blocks)",
                headers.len() - starting_index
            )),
        );

        for (_index, header) in headers.iter().enumerate().skip(starting_index) {
            if i > (headers.len() - starting_index) / 10 {
                percentage += 10;
                send_log(
                    logger_sender,
                    Log::Message(format!("Utxo generation is ({percentage}%) completed...")),
                );
                i = 0;
            }
            let path = format!("{}/blocks/{}.bin", self.store_path, header.hash_as_string());
            let block = match Block::restore(path) {
                Ok(block) => block,
                Err(_) => {
                    send_log(
                        logger_sender,
                        Log::Message(String::from(
                            "Error generating UTXO (block file broken), please restart the app.",
                        )),
                    );
                    exit(0);
                }
            };
            self.update_from_block(&block, false)?;
            drop(block);
            last_block_hash = header.hash().clone();
            i += 1;
        }
        Ok(last_block_hash)
    }

    fn serialize(&mut self, block_hash: Vec<u8>) -> Vec<u8> {
        let mut buffer = vec![];
        buffer.extend(block_hash);
        buffer.extend((self.tx_set.len() as u64).to_le_bytes());

        for (out_point, value) in &self.tx_set {
            buffer.extend(out_point.serialize());
            buffer.extend(value.tx_out.serialize());
            buffer.extend(value.block_hash.clone());
            buffer.extend(value.block_timestamp.to_le_bytes());
        }
        buffer
    }

    pub fn parse(buffer: Vec<u8>) -> Result<(Vec<u8>, HashMap<OutPoint, UTXOValue>), CustomError> {
        let mut parser = BufferParser::new(buffer);

        let last_block_hash = parser.extract_buffer(32)?.to_vec();
        let tx_set_len = parser.extract_u64()? as usize;
        let mut tx_set: HashMap<OutPoint, UTXOValue> = HashMap::new();

        for _i in 0..tx_set_len {
            let out_point = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;

            let value = UTXOValue {
                tx_out: TransactionOutput::parse(&mut parser)?,
                block_hash: parser.extract_buffer(32)?.to_vec(),
                block_timestamp: parser.extract_u32()?,
            };
            tx_set.insert(out_point, value);
        }

        Ok((last_block_hash, tx_set))
    }

    /// Actualiza las UTXO a partir de un bloque, eliminando los outputs gastados y agregando los nuevos outputs.
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
                    block_hash: block.header.hash().clone(),
                    block_timestamp: block.header.timestamp,
                };
                self.tx_set.insert(out_point.clone(), value);
            }
        }

        if save {
            self.save(block.header.hash().clone())?;
        }

        Ok(())
    }

    fn save(&mut self, block_hash: Vec<u8>) -> Result<(), CustomError> {
        let buffer = self.serialize(block_hash);

        let path = format!("{}/{}", self.store_path, self.path);
        if Path::new(&path).exists() {
            remove_file(path.clone())?;
        }
        let mut file = open_new_file(path, false)?;

        file.write_all(&buffer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::fs;

    use chrono::Local;
    use gtk::glib::{self, Priority};

    use crate::{
        logger::Logger, messages::transaction::Transaction, structs::tx_input::TransactionInput,
        wallet::get_script_pubkey,
    };

    use super::*;

    #[test]
    fn test_save_restore() {
        let filename = format!("{}", Local::now());
        let store_path = String::from("tests");
        let mut utxo_set = UTXO::new(store_path.clone(), filename.clone()).unwrap();

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
            block_timestamp: 1680000000,
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
            block_timestamp: 1680000001,
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
            block_timestamp: 1680000002,
        };
        utxo_set.tx_set.insert(key1, value1);
        utxo_set.tx_set.insert(key2, value2);
        utxo_set.tx_set.insert(key3, value3);

        assert_eq!(utxo_set.tx_set.len(), 3);

        utxo_set
            .save(vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24, 25, 26, 27, 28, 29, 30, 31, 32,
            ])
            .unwrap();

        let mut utxo_set2 = UTXO::new(store_path.clone(), filename.clone()).unwrap();
        utxo_set2.restore_utxo().unwrap();

        assert_eq!(utxo_set2.tx_set.len(), 3);
        assert_eq!(utxo_set2.tx_set, utxo_set.tx_set);

        fs::remove_file(format!("{}/{}", store_path, filename)).unwrap();
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
            hash: vec![],
            block_downloaded: true,
            broadcasted: true,
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
            hash: vec![],
            block_downloaded: true,
            broadcasted: true,
        };

        assert_eq!(
            calculate_index_from_timestamp(&vec![header1.clone(), header2.clone()], 2),
            1
        );
        assert_eq!(
            calculate_index_from_timestamp(&vec![header1.clone(), header2.clone()], 1),
            0
        );
        assert_eq!(
            calculate_index_from_timestamp(&vec![header1.clone(), header2.clone()], 3),
            1
        );
        assert_eq!(
            calculate_index_from_timestamp(&vec![header1, header2], 0),
            0
        );
    }

    #[test]
    fn utxo_serialization_and_parsing() {
        let filename = String::from("test_utxo.bin");
        let store_path = String::from("tests");
        let mut utxo_set = UTXO::new(store_path, filename.clone()).unwrap();
        let block_hash = vec![
            127, 47, 239, 163, 175, 36, 146, 56, 212, 168, 146, 23, 101, 29, 205, 186, 7, 67, 240,
            23, 75, 32, 175, 14, 221, 106, 150, 247, 21, 243, 205, 109,
        ];

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
            block_hash: block_hash.clone(),
            block_timestamp: 1680000000,
        };
        utxo_set.tx_set.insert(key, value);

        let buffer = utxo_set.serialize(block_hash.clone());
        let (last_block_hash, parsed_tx_set) = UTXO::parse(buffer).unwrap();
        assert_eq!(last_block_hash, block_hash);
        assert_eq!(utxo_set.tx_set, parsed_tx_set);
    }

    #[test]
    fn utxo_generation() {
        let (gui_sender, _gui_receiver) = glib::MainContext::channel(Priority::default());

        let logger = Logger::new(&String::from("tests/test_log.txt"), gui_sender).unwrap();
        let logger_sender = logger.get_sender();

        let path = format!("tests/test_block.bin");

        let block_old = Block {
            header: BlockHeader {
                bits: 486604799,
                block_downloaded: true,
                broadcasted: true,
                hash: vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6,
                    7, 8, 9, 0, 1, 2,
                ],
                merkle_root: vec![],
                nonce: 409655068,
                prev_block_hash: vec![],
                timestamp: 1680000000,
                version: 21123123,
            },
            transactions: vec![Transaction {
                inputs: vec![TransactionInput {
                    previous_output: OutPoint {
                        hash: vec![],
                        index: 0,
                    },
                    script_sig: vec![],
                    sequence: 0,
                }],
                outputs: vec![TransactionOutput {
                    script_pubkey: vec![1, 2],
                    value: 100,
                }],
                lock_time: 0,
                version: 0,
            }],
        };

        // bloque con 42 inputs y outputs en 20 txs
        let block = Block::restore(path).unwrap();

        if Path::new("tests/test_utxo.bin").exists() {
            fs::remove_file("tests/test_utxo.bin").unwrap();
        }
        let filename = String::from("test_utxo.bin");
        let store_path = String::from("tests");
        let mut utxo_set = UTXO::new(store_path, filename.clone()).unwrap();

        let headers = vec![block_old.header.clone(), block.header.clone()];
        utxo_set
            .generate(&headers, &mut logger_sender.clone())
            .unwrap();

        // // solo tienen que estar los utxo del segundo bloque
        // assert_eq!(utxo_set.tx_set.len(), 42);
        // assert_eq!(utxo_set.is_synced(), true);

        // fs::remove_file("tests/test_log.txt").unwrap();
        // fs::remove_file("tests/test_utxo.bin").unwrap();
    }

    #[test]
    fn wallet_utxo_generation() {
        let filename = String::from("test_utxo.bin");
        let store_path = String::from("tests");
        let mut utxo_set = UTXO::new(store_path, filename.clone()).unwrap();
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
            block_timestamp: 1680000000,
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
            block_timestamp: 1680000001,
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
            block_timestamp: 1680000002,
        };
        utxo_set.tx_set.insert(key1.clone(), value1.clone());
        utxo_set.tx_set.insert(key2.clone(), value2.clone());
        utxo_set.tx_set.insert(key3.clone(), value3);
        assert!(utxo_set.generate_wallet_utxo(&wallet).unwrap().len() == 2);
        assert!(utxo_set
            .generate_wallet_utxo(&wallet)
            .unwrap()
            .contains(&(key1, value1)));
        assert!(utxo_set
            .generate_wallet_utxo(&wallet)
            .unwrap()
            .contains(&(key2, value2)));
    }

    #[test]
    fn correct_wallet_balance() {
        let filename = String::from("test_utxo.bin");
        let store_path = String::from("tests");
        let mut utxo_set = UTXO::new(store_path, filename.clone()).unwrap();

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
            block_timestamp: 1680000000,
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
            block_timestamp: 1680000001,
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
            block_timestamp: 1680000002,
        };
        utxo_set.tx_set.insert(key1, value1);
        utxo_set.tx_set.insert(key2, value2);
        utxo_set.tx_set.insert(key3, value3);
        assert_eq!(utxo_set.wallet_balance(&wallet).unwrap(), 300);
    }
}
