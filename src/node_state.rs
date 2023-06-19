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
    utxo::{parse_utxo, serialize_utxo},
    wallet::Wallet,
};

const START_DATE_IBD: u32 = 1681095630;

pub struct NodeState {
    logger_sender: mpsc::Sender<Log>,
    gui_sender: Sender<GUIActions>,
    headers_file: File,
    utxo_file: File,
    headers: Vec<BlockHeader>,
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
        let mut headers_file = open_new_file(String::from("store/headers.bin"), true)?;

        let mut saved_headers_buffer = vec![];
        headers_file.read_to_end(&mut saved_headers_buffer)?;

        let headers = match Headers::parse_headers(saved_headers_buffer) {
            Ok(headers) => headers,
            Err(_) => vec![],
        };

        let mut utxo_file = open_new_file(String::from("store/utxo.bin"), true)?;
        let mut saved_utxo_buffer = vec![];
        utxo_file.read_to_end(&mut saved_utxo_buffer)?;
        let utxo_set = parse_utxo(saved_utxo_buffer)?;
        let utxo_sync = if utxo_set.len() > 0 { true } else { false };

        let wallets = restore_wallets()?;

        let pending_blocks_ref = Arc::new(Mutex::new(HashMap::new()));

        let node_state_ref = Arc::new(Mutex::new(Self {
            logger_sender,
            gui_sender,
            headers_file,
            utxo_file,
            headers,
            wallets,
            active_wallet: None,
            pending_blocks_ref,
            utxo_set,
            pending_tx_set: HashMap::new(),
            headers_sync: false,
            blocks_sync: false,
            utxo_sync,
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

    fn update_new_block_transactions(&mut self, block: Block) -> Result<(), CustomError> {
        update_transaction_sets(
            &mut self.utxo_set,
            &mut self.utxo_file,
            &mut self.pending_tx_set,
            block,
            &mut self.wallets,
            true,
        )?;
        if let Some(_wallet) = &self.active_wallet {
            self.gui_sender
                .send(GUIActions::NewBlock)
                .map_err(|_| CustomError::CannotInitGUI)?;
        }
        Ok(())
    }

    fn sync_up_utxo(&mut self) {
        self.verify_blocks_sync().unwrap_or_else(|_| {
            send_log(
                &self.logger_sender,
                Log::Message("Error verifying blocks synchronization".to_string()),
            );
        });
    }

    pub fn append_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let filename = hash_as_string(block_hash.clone());
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename), true)?;
        block_file.write_all(&block.serialize())?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove(&block_hash);
        drop(pending_blocks);

        match self.utxo_sync {
            true => self.update_new_block_transactions(block)?,
            false => self.sync_up_utxo(),
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
            if !self.is_utxo_sync() {
                self.generate_utxo()?;
            }
            self.gui_sender
                .send(GUIActions::NodeStateReady)
                .map_err(|_| CustomError::CannotInitGUI)?;
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
            let mut block_file = open_new_file(format!("store/blocks/{}.bin", hash), true)?;
            let mut block_buffer = Vec::new();
            block_file.read_to_end(&mut block_buffer)?;
            let block = Block::parse(block_buffer)?;
            update_transaction_sets(
                &mut self.utxo_set,
                &mut self.utxo_file,
                &mut self.pending_tx_set,
                block,
                &mut self.wallets,
                false,
            )
            .unwrap();
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
        //no se si ese custom va, o hacemos uno especifico?
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
        if public_key.len() != 34 {
            return Err(CustomError::Validation(
                "Public key must be 34 characters long".to_string(),
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

        let new_wallet = Wallet::new(name, public_key, private_key, &self.utxo_set)?;
        self.wallets.push(new_wallet);
        save_wallets(&mut self.wallets)?;
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

    pub fn append_pending_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), CustomError> {
        let tx_hash = transaction.hash();

        if !self.pending_tx_set.contains_key(&tx_hash) {
            if let Some(_wallet) = &self.active_wallet {
                self.gui_sender
                    .send(GUIActions::NewPendingTx)
                    .map_err(|_| CustomError::CannotInitGUI)?;
            }
            self.pending_tx_set.insert(tx_hash, transaction);
        }
        //self.pending_tx_set.entry(tx_hash).or_insert(transaction);
        Ok(())
    }

    pub fn get_pending_tx_from_wallet(
        &self,
    ) -> Result<HashMap<OutPoint, TransactionOutput>, CustomError> {
        let mut pending_transactions = HashMap::new();
        let active_wallet = match self.get_active_wallet() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let pubkey_hash = active_wallet.get_pubkey_hash()?;

        for (tx_hash, tx) in self.pending_tx_set.iter() {
            for (index, tx_out) in tx.outputs.iter().enumerate() {
                if tx_out.is_sent_to_key(&pubkey_hash) {
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

    pub fn make_transaction(
        &self,
        mut outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<(), CustomError> {
        let active_wallet = match self.get_active_wallet() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let mut total_value = fee;
        for output in outputs.values() {
            total_value += output;
        }
        let wallet_balance = self.get_balance()?;
        if total_value > wallet_balance {
            send_log(
                &self.logger_sender,
                Log::Error(CustomError::Validation(
                    "Insufficient funds to make transaction".to_string(),
                )),
            );
            return Err(CustomError::InsufficientFunds);
        }

        println!("CHECK 1");
        let mut active_wallet_utxo = self.get_active_wallet_utxo()?;

        active_wallet_utxo.sort_by(|a, b| b.1.value.cmp(&a.1.value));
        let mut inputs = vec![];
        let mut total_input_value = 0;
        for (out_point, tx_out) in active_wallet_utxo.iter() {
            inputs.push(out_point.clone());
            total_input_value += tx_out.value;
            if total_input_value >= total_value {
                break;
            }
        }
        let change = total_input_value - total_value;
        if change > 0 {
            outputs.insert(active_wallet.pubkey.clone(), change);
        }
        Transaction::create(active_wallet, inputs, outputs)
    }

    fn get_active_wallet_utxo(&self) -> Result<Vec<(OutPoint, TransactionOutput)>, CustomError> {
        let active_wallet = match self.get_active_wallet() {
            Some(active_wallet) => active_wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let pubkey_hash = active_wallet.get_pubkey_hash()?;

        let mut active_wallet_utxo = vec![];
        for (out_point, tx_out) in self.utxo_set.iter() {
            if tx_out.is_sent_to_key(&pubkey_hash) {
                active_wallet_utxo.push((out_point.clone(), tx_out.clone()));
            }
        }
        Ok(active_wallet_utxo)
    }
}

fn save_wallets(wallets: &mut Vec<Wallet>) -> Result<(), CustomError> {
    let mut wallets_file = open_new_file(String::from("store/wallets.bin"), false)?;
    let mut wallets_buffer = vec![];
    for wallet in wallets.iter() {
        wallets_buffer.append(&mut wallet.serialize());
    }
    wallets_file.write_all(&wallets_buffer)?;
    Ok(())
}

fn restore_wallets() -> Result<Vec<Wallet>, CustomError> {
    let mut wallets_file = open_new_file(String::from("store/wallets.bin"), false)?;
    let mut saved_wallets_buffer = vec![];
    wallets_file.read_to_end(&mut saved_wallets_buffer)?;
    let wallets = match Wallet::parse_wallets(saved_wallets_buffer) {
        Ok(wallets) => wallets,
        Err(_) => vec![],
    };
    Ok(wallets)
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

fn update_wallet_movements(
    wallets: &mut Vec<Wallet>,
    utxo_set: &mut HashMap<OutPoint, TransactionOutput>,
    tx: &Transaction,
    block: &Block,
    wallets_updated: &mut bool,
) -> Result<(), CustomError> {
    for wallet in wallets.into_iter() {
        let movement = tx.get_movement(&wallet.get_pubkey_hash()?, utxo_set);
        if let Some(mut movement) = movement {
            movement.block_hash = Some(block.header.hash());
            wallet.update_history(movement);
            *wallets_updated = true;
        }
    }
    Ok(())
}
fn update_utxo_set(
    tx: &Transaction,
    utxo_set: &mut HashMap<OutPoint, TransactionOutput>,
    utxo_file: &mut File,
) -> Result<(), CustomError> {
    for tx_in in tx.inputs.iter() {
        utxo_set.remove(&tx_in.previous_output);
    }
    for (index, tx_out) in tx.outputs.iter().enumerate() {
        let out_point = OutPoint {
            hash: tx.hash().clone(),
            index: index as u32,
        };
        utxo_set.insert(out_point.clone(), tx_out.clone());
        utxo_file.write_all(&serialize_utxo(&out_point, &tx_out).to_vec())?;
    }
    Ok(())
}

fn update_transaction_sets(
    utxo_set: &mut HashMap<OutPoint, TransactionOutput>,
    utxo_file: &mut File,
    pending_tx_set: &mut HashMap<Vec<u8>, Transaction>,
    block: Block,
    wallets: &mut Vec<Wallet>,
    is_utxo_generated: bool,
) -> Result<(), CustomError> {
    let mut wallets_updated = false;
    for tx in block.transactions.iter() {
        update_utxo_set(tx, utxo_set, utxo_file)?;
        if is_utxo_generated {
            update_wallet_movements(wallets, utxo_set, tx, &block, &mut wallets_updated)?;
        }
        if pending_tx_set.contains_key(&tx.hash()) {
            pending_tx_set.remove(&tx.hash());
        }
    }
    if wallets_updated {
        save_wallets(wallets)?;
    }
    Ok(())
}

pub fn get_current_timestamp() -> Result<u64, CustomError> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}
