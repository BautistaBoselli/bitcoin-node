use std::{
    collections::{hash_map, HashMap},
    fs::{File, OpenOptions},
    io::Write,
    sync::{mpsc, Arc, Mutex},
    time::SystemTime,
};

use gtk::glib::Sender;

use crate::{
    error::CustomError,
    gui::init::GUIActions,
    logger::{send_log, Log},
    messages::{
        block::Block,
        headers::{BlockHeader, Headers},
        transaction::{OutPoint, Transaction, TransactionOutput},
    },
    states::{utxo_state::UTXO, wallets_state::Wallets},
    wallet::Wallet,
};

pub struct NodeState {
    logger_sender: mpsc::Sender<Log>,
    gui_sender: Sender<GUIActions>,
    headers_file: File,
    headers: Vec<BlockHeader>,
    wallets: Wallets,
    pending_blocks_ref: Arc<Mutex<HashMap<Vec<u8>, u64>>>,
    utxo: UTXO,
    pending_tx_set: HashMap<Vec<u8>, Transaction>,
    headers_sync: bool,
    blocks_sync: bool,
}

impl NodeState {
    pub fn new(
        logger_sender: mpsc::Sender<Log>,
        gui_sender: Sender<GUIActions>,
    ) -> Result<Arc<Mutex<Self>>, CustomError> {
        let mut headers_file = open_new_file(String::from("store/headers.bin"), true)?;
        let headers = BlockHeader::restore_headers(&mut headers_file)?;

        let wallets = Wallets::new(String::from("store/wallets.bin"))?;

        let pending_blocks_ref = Arc::new(Mutex::new(HashMap::new()));

        let node_state_ref = Arc::new(Mutex::new(Self {
            logger_sender,
            gui_sender,
            headers_file,
            headers,
            wallets,
            pending_blocks_ref,
            utxo: UTXO::new(String::from("store/utxo.bin"))?,
            pending_tx_set: HashMap::new(),
            headers_sync: false,
            blocks_sync: false,
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

        self.verify_headers_sync(headers_count)?;
        self.verify_sync()
    }

    pub fn append_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let path = format!("store/blocks/{}.bin", block.header.hash_as_string());
        block.save(path)?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove(&block_hash);
        drop(pending_blocks);

        self.verify_sync()?;

        if self.is_synced() {
            self.utxo.update_from_block(&block, true)?;
        } else if self.blocks_sync {
            self.utxo.generate(&self.headers, &mut self.logger_sender)?;
        }

        self.update_pending_tx(&block)?;
        self.update_wallets(&block)?;

        Ok(())
    }

    pub fn append_pending_block(&mut self, header_hash: Vec<u8>) -> Result<(), CustomError> {
        let current_time = get_current_timestamp()?;
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.insert(header_hash, current_time);
        drop(pending_blocks);

        Ok(())
    }

    fn remove_pending_blocks(&self) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.drain();
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

    pub fn update_pending_tx(&mut self, block: &Block) -> Result<(), CustomError> {
        for tx in block.transactions.iter() {
            if self.pending_tx_set.contains_key(&tx.hash()) {
                self.pending_tx_set.remove(&tx.hash());
            }
        }

        Ok(())
    }

    pub fn update_wallets(&mut self, block: &Block) -> Result<(), CustomError> {
         let wallets_updated = self.wallets.update(&block, &self.utxo)?;
        if wallets_updated {
                self.gui_sender
                    .send(GUIActions::WalletsUpdated)
                    .map_err(|_| CustomError::CannotInitGUI)?;
        }
        Ok(())
    }

    pub fn is_synced(&self) -> bool {
        self.headers_sync && self.blocks_sync && self.utxo.is_synced()
    }

    pub fn is_blocks_sync(&self) -> bool {
        self.blocks_sync
    }

    pub fn is_block_pending(&self, block_hash: &Vec<u8>) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.contains_key(block_hash))
    }

    fn is_pending_blocks_empty(&self) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_empty())
    }

    pub fn verify_sync(&mut self) -> Result<(), CustomError> {
        if self.headers_sync {
            self.verify_blocks_sync()?;
        }

        if self.blocks_sync && !self.utxo.is_synced() {
            self.utxo.generate(&self.headers, &mut self.logger_sender)?;
        }

        if self.is_synced() {
            self.gui_sender
                .send(GUIActions::NodeStateReady)
                .map_err(|_| CustomError::CannotInitGUI)?;
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
        }
        Ok(())
    }

    pub fn get_wallets(&self) -> &Vec<Wallet> {
        &self.wallets.get_all()
    }

    pub fn append_wallet(
        &mut self,
        name: String,
        public_key: String,
        private_key: String,
    ) -> Result<(), CustomError> {
        let new_wallet = Wallet::new(name, public_key, private_key, &self.utxo)?;
        self.wallets.append(new_wallet)
    }

    pub fn change_wallet(&mut self, public_key: String) -> Result<(), CustomError> {
        self.wallets.set_active(public_key)?;
        self.gui_sender.send(GUIActions::WalletChanged)?;
        Ok(())
    }

    pub fn get_active_wallet(&self) -> Option<&Wallet> {
        self.wallets.get_active()
    }

    pub fn get_active_wallet_balance(&self) -> Result<u64, CustomError> {
        let active_wallet = match self.wallets.get_active() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };

        self.utxo.wallet_balance(active_wallet)
    }

    pub fn get_active_wallet_utxo(
        &self,
    ) -> Result<Vec<(OutPoint, TransactionOutput)>, CustomError> {
        let active_wallet = match self.wallets.get_active() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };

        self.utxo.generate_wallet_utxo(active_wallet)
    }

    pub fn get_active_wallet_pending_txs(
        &self,
    ) -> Result<HashMap<OutPoint, TransactionOutput>, CustomError> {
        let mut pending_transactions = HashMap::new();
        let active_wallet = match self.wallets.get_active() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let pubkey_hash = active_wallet.get_pubkey_hash()?;

        for (tx_hash, tx) in self.pending_tx_set.iter() {
            for (index, tx_out) in tx.outputs.iter().enumerate() {
                if tx_out.is_sent_to_key(&pubkey_hash)? {
                    let out_point = OutPoint {
                        hash: tx_hash.clone(),
                        index: index as u32,
                    };
                    pending_transactions.insert(out_point, tx_out.clone());
                }
            }
        }
        Ok(pending_transactions)
    }

    pub fn append_pending_tx(&mut self, transaction: Transaction) -> Result<(), CustomError> {
        let tx_hash = transaction.hash();

        if let hash_map::Entry::Vacant(e) = self.pending_tx_set.entry(tx_hash) {
            if let Some(_wallet) = &self.wallets.get_active() {
                self.gui_sender
                    .send(GUIActions::NewPendingTx)
                    .map_err(|_| CustomError::CannotInitGUI)?;
            }
            e.insert(transaction);
        }

        Ok(())
    }

    pub fn make_transaction(
        &mut self,
        mut outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<Transaction, CustomError> {
        let active_wallet = match self.get_active_wallet() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let total_value = self.calculate_total_value(fee, &outputs)?;
        let mut active_wallet_utxo = self.get_active_wallet_utxo()?;

        active_wallet_utxo.sort_by(|a, b| b.1.value.cmp(&a.1.value));
        let (inputs, total_input_value) = calculate_inputs(active_wallet_utxo, total_value);

        let change = total_input_value - total_value;
        if change > 0 {
            outputs.insert(active_wallet.pubkey.clone(), change);
        }

        Transaction::create(active_wallet, inputs, outputs)
    }

    fn calculate_total_value(&self, fee: u64, outputs: &HashMap<String, u64>) -> Result<u64, CustomError> {
        let mut total_value = fee;
        for output in outputs.values() {
            total_value += output;
        }
        let wallet_balance = self.get_active_wallet_balance()?;
        if total_value > wallet_balance {
            send_log(
                &self.logger_sender,
                Log::Error(CustomError::Validation(
                    "Insufficient funds to make transaction".to_string(),
                )),
            );
            return Err(CustomError::InsufficientFunds);
        }
        Ok(total_value)
    }
}

fn calculate_inputs(active_wallet_utxo: Vec<(OutPoint, TransactionOutput)>, total_value: u64) -> (Vec<OutPoint>, u64) {
    let mut inputs = vec![];
    let mut total_input_value = 0;
    for (out_point, tx_out) in active_wallet_utxo.iter() {
        inputs.push(out_point.clone());
        total_input_value += tx_out.value;
        if total_input_value >= total_value {
            break;
        }
    }
    (inputs, total_input_value)
}

pub fn open_new_file(path_to_file: String, append: bool) -> Result<std::fs::File, CustomError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(append)
        .open(path_to_file)?;
    Ok(file)
}

pub fn get_current_timestamp() -> Result<u64, CustomError> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}
