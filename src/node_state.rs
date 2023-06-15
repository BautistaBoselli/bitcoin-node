use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    sync::{mpsc, Arc, Mutex},
    time::SystemTime,
};

use gtk::glib::Sender;

use crate::{
    error::CustomError,
    gui::init::GUIActions,
    logger::{send_log, Log},
    message::Message,
    messages::{
        block::Block,
        headers::{hash_as_string, BlockHeader, Headers},
        transaction::{OutPoint, Transaction, TransactionOutput},
    },
    wallet::Wallet,
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeState {
    logger_sender: mpsc::Sender<Log>,
    gui_sender: Sender<GUIActions>,
    headers_file: File,
    headers: Vec<BlockHeader>,
    wallets_file: File,
    wallets: Vec<Wallet>,
    active_wallet: Option<String>,
    pending_blocks_ref: Arc<Mutex<HashMap<Vec<u8>, u64>>>,
    utxo_set: HashMap<OutPoint, TransactionOutput>,
    pending_tx_set: HashMap<Vec<u8>, Transaction>,
    headers_sync: bool,
    blocks_sync: bool,
    utxo_sync: bool,
}

impl NodeState {
    pub fn new(
        logger_sender: mpsc::Sender<Log>,
        gui_sender: Sender<GUIActions>,
    ) -> Result<Arc<Mutex<Self>>, CustomError> {
        let mut headers_file = open_new_file(String::from("store/headers.bin"))?;

        let mut saved_headers_buffer = vec![];
        headers_file.read_to_end(&mut saved_headers_buffer)?;

        let headers = match Headers::parse_headers(saved_headers_buffer) {
            Ok(headers) => headers,
            Err(_) => vec![],
        };

        let mut wallets_file = open_new_file(String::from("store/wallets.bin"))?;

        let mut saved_wallets_buffer = vec![];
        wallets_file.read_to_end(&mut saved_wallets_buffer)?;

        let wallets = match Wallet::parse_wallets(saved_wallets_buffer) {
            Ok(wallets) => wallets,
            Err(_) => vec![],
        };

        let pending_blocks_ref = Arc::new(Mutex::new(HashMap::new()));

        let node_state_ref = Arc::new(Mutex::new(Self {
            logger_sender,
            gui_sender,
            headers_file,
            headers,
            wallets_file,
            wallets,
            active_wallet: None,
            pending_blocks_ref,
            utxo_set: HashMap::new(),
            pending_tx_set: HashMap::new(),
            headers_sync: false,
            blocks_sync: false,
            utxo_sync: false,
        }));

        Ok(node_state_ref)
    }

    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.last().map(|header| header.hash())
    }

    pub fn append_headers(&mut self, headers: &mut Headers) -> Result<(), CustomError> {
        self.headers_file.write_all(&headers.serialize_headers())?;
        let headers_count = headers.headers.len();

        self.headers.append(&mut headers.headers);

        send_log(
            &self.logger_sender,
            Log::Message(format!(
                "There are {} headers, new {}",
                self.headers.len(),
                headers_count
            )),
        );

        self.verify_headers_sync(headers_count)
    }

    fn is_pending_blocks_empty(&self) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_empty())
    }

    fn remove_pending_blocks(&self) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.drain();
        Ok(())
    }

    pub fn append_pending_block(&mut self, header_hash: Vec<u8>) -> Result<(), CustomError> {
        let current_time = get_current_timestamp()?;
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.insert(header_hash, current_time);
        drop(pending_blocks);

        Ok(())
    }

    pub fn get_stale_block_downloads(&self) -> Result<Vec<Vec<u8>>, CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        let mut to_remove = Vec::new();

        for (block_hash, timestamp) in pending_blocks.iter() {
            if *timestamp + 5 < get_current_timestamp()? {
                to_remove.push(block_hash.clone());
            }
        }

        for block_hash in to_remove.iter() {
            pending_blocks.remove(block_hash);
        }

        Ok(to_remove)
    }

    pub fn append_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let filename = hash_as_string(block_hash.clone());
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename))?;
        block_file.write_all(&block.serialize())?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove(&block_hash);
        drop(pending_blocks);

        match self.utxo_sync {
            true => update_transaction_sets(&mut self.utxo_set, &mut self.pending_tx_set, block),
            false => self.verify_blocks_sync().unwrap_or_else(|_| {
                send_log(
                    &self.logger_sender,
                    Log::Message("Error verifying blocks synchronization".to_string()),
                );
            }),
        }

        Ok(())
    }

    fn verify_headers_sync(&mut self, new_headers_count: usize) -> Result<(), CustomError> {
        if self.headers_sync {
            return Ok(());
        }

        self.headers_sync = new_headers_count < 2000;
        if self.headers_sync {
            send_log(
                &self.logger_sender,
                Log::Message("headers sync completed".to_string()),
            );
            self.verify_blocks_sync()?;
        }
        Ok(())
    }

    fn verify_blocks_sync(&mut self) -> Result<(), CustomError> {
        if self.blocks_sync {
            return Ok(());
        }

        let pending_blocks_empty = self.is_pending_blocks_empty()?;

        self.blocks_sync = self.headers_sync && pending_blocks_empty;

        if self.blocks_sync {
            self.remove_pending_blocks()?;
            send_log(
                &self.logger_sender,
                Log::Message("blocks sync completed".to_string()),
            );

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

        send_log(
            &self.logger_sender,
            Log::Message("Beginning the generation of the utxo (0%)...".to_string()),
        );

        let mut i = 0;
        let mut percentage = 0;
        for header in self.headers.iter().rev().take(blocks_after_timestamp).rev() {
            if i > blocks_after_timestamp / 10 {
                percentage += 10;
                send_log(
                    &self.logger_sender,
                    Log::Message(format!(
                        "The generation of utxo is ({}%) completed...",
                        percentage
                    )),
                );
                i = 0;
            }
            let hash = header.hash_as_string();
            let mut block_file = open_new_file(format!("store/blocks/{}.bin", hash))?;
            let mut block_buffer = Vec::new();
            block_file.read_to_end(&mut block_buffer)?;
            let block = Block::parse(block_buffer)?;
            update_transaction_sets(&mut self.utxo_set, &mut self.pending_tx_set, block);
            i += 1;
        }
        self.utxo_sync = true;
        send_log(
            &self.logger_sender,
            Log::Message("The generation of utxo is (100%) completed".to_string()),
        );
        send_log(
            &self.logger_sender,
            Log::Message("Utxo generation is finished".to_string()),
        );
        Ok(())
    }

    pub fn is_headers_sync(&self) -> bool {
        self.headers_sync
    }

    pub fn is_blocks_sync(&self) -> bool {
        self.blocks_sync
    }

    pub fn is_block_pending(&self, block_hash: &Vec<u8>) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.contains_key(block_hash))
    }

    pub fn is_utxo_sync(&self) -> bool {
        self.utxo_sync
    }

    pub fn number_of_headers(&self) -> usize {
        self.headers.len()
    }

    pub fn get_wallets(&self) -> &Vec<Wallet> {
        &self.wallets
    }

    pub fn append_wallet(
        &mut self,
        name: String,
        public_key: String,
        private_key: String,
    ) -> Result<(), CustomError> {
        if name.is_empty() || public_key.is_empty() || private_key.is_empty() {
            return Err(CustomError::Validation(
                "Name, public key and private key must not be empty".to_string(),
            ));
        }
        if public_key.len() != 35 {
            return Err(CustomError::Validation(
                "Public key must be 35 characters long".to_string(),
            ));
        }
        if self
            .wallets
            .iter()
            .any(|wallet| wallet.pubkey == public_key)
        {
            return Err(CustomError::Validation(
                "Public key already exists".to_string(),
            ));
        }

        let new_wallet = Wallet::new(name, public_key, private_key);
        self.wallets_file.write_all(&new_wallet.serialize())?;
        self.wallets.push(new_wallet);
        Ok(())
    }

    pub fn change_wallet(&mut self, public_key: String) {
        self.active_wallet = self
            .wallets
            .iter()
            .find(|wallet| wallet.pubkey == public_key)
            .map(|wallet| wallet.pubkey.clone());
        self.gui_sender.send(GUIActions::WalletChanged).unwrap();
    }

    pub fn get_active_wallet(&self) -> Option<&Wallet> {
        match self.active_wallet {
            Some(ref active_wallet) => self
                .wallets
                .iter()
                .find(|wallet| wallet.pubkey == *active_wallet),
            None => None,
        }
    }

    pub fn get_balance(&self) -> Result<u64, CustomError> {
        let mut balance = 0;

        let active_wallet = match self.get_active_wallet() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let pubkey_hash = active_wallet.get_pubkey_hash()?;

        for (_, tx_out) in self.utxo_set.iter() {
            if tx_out.is_sent_to_key(&pubkey_hash) {
                balance += tx_out.value;
            }
        }
        Ok(balance)
    }

    pub fn append_pending_transaction(&mut self, transaction: Transaction) {
        let tx_hash = transaction.hash();

        self.pending_tx_set.entry(tx_hash).or_insert(transaction);
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

fn update_transaction_sets(
    utxo_set: &mut HashMap<OutPoint, TransactionOutput>,
    pending_tx_set: &mut HashMap<Vec<u8>, Transaction>,
    block: Block,
) {
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
        if pending_tx_set.contains_key(&tx.hash()) {
            pending_tx_set.remove(&tx.hash());
        }
    }
}

pub fn get_current_timestamp() -> Result<u64, CustomError> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}
