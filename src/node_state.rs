use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

use gtk::glib::Sender;

use crate::{
    error::CustomError,
    gui::init::GUIEvents,
    logger::{send_log, Log},
    messages::{
        block::Block,
        headers::Headers,
        transaction::{OutPoint, Transaction, TransactionOutput},
    },
    states::{
        headers_state::HeadersState, pending_blocks::PendingBlocks, pending_txs::PendingTxs,
        utxo_state::UTXO, wallets_state::Wallets,
    },
    wallet::Wallet,
};

pub struct NodeState {
    logger_sender: mpsc::Sender<Log>,
    gui_sender: Sender<GUIEvents>,
    headers: HeadersState,
    wallets: Wallets,
    pending_blocks_ref: Arc<Mutex<PendingBlocks>>,
    utxo: UTXO,
    pending_txs: PendingTxs,
    blocks_sync: bool,
}

impl NodeState {
    pub fn new(
        logger_sender: mpsc::Sender<Log>,
        gui_sender: Sender<GUIEvents>,
    ) -> Result<Arc<Mutex<Self>>, CustomError> {
        let node_state_ref = Arc::new(Mutex::new(Self {
            logger_sender: logger_sender.clone(),
            gui_sender,
            headers: HeadersState::new(String::from("store/headers.bin"), logger_sender)?,
            wallets: Wallets::new(String::from("store/wallets.bin"))?,
            pending_blocks_ref: PendingBlocks::new(),
            utxo: UTXO::new(String::from("store/utxo.bin"))?,
            pending_txs: PendingTxs::new(),
            blocks_sync: false,
        }));

        Ok(node_state_ref)
    }

    pub fn append_block(&mut self, block_hash: Vec<u8>, block: Block) -> Result<(), CustomError> {
        let path = format!("store/blocks/{}.bin", block.header.hash_as_string());
        block.save(path)?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove_block(&block_hash)?;
        drop(pending_blocks);

        self.verify_sync()?;

        if self.is_synced() {
            self.utxo.update_from_block(&block, true)?;
        }

        self.update_pending_tx(&block)?;
        self.update_wallets(&block)?;

        Ok(())
    }

    // Headers
    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.get_last_header_hash()
    }

    pub fn append_headers(&mut self, headers: &mut Headers) -> Result<(), CustomError> {
        self.headers.append_headers(headers)?;
        self.verify_sync()
    }

    // Sync

    pub fn is_synced(&self) -> bool {
        self.headers.is_synced() && self.blocks_sync && self.utxo.is_synced()
    }

    pub fn is_blocks_sync(&self) -> bool {
        self.blocks_sync
    }

    pub fn verify_sync(&mut self) -> Result<(), CustomError> {
        if self.headers.is_synced() {
            self.verify_blocks_sync()?;
        }

        if self.blocks_sync && !self.utxo.is_synced() {
            self.utxo
                .generate(self.headers.get_all(), &mut self.logger_sender)?;
        }

        if self.is_synced() {
            self.gui_sender
                .send(GUIEvents::NodeStateReady)
                .map_err(|_| CustomError::CannotInitGUI)?;
        }

        Ok(())
    }

    fn verify_blocks_sync(&mut self) -> Result<(), CustomError> {
        if self.blocks_sync {
            return Ok(());
        }

        let pending_blocks_empty = self.is_pending_blocks_empty()?;

        self.blocks_sync = self.headers.is_synced() && pending_blocks_empty;

        if self.blocks_sync {
            self.remove_pending_blocks()?;
            send_log(
                &self.logger_sender,
                Log::Message("blocks sync completed".to_string()),
            );
        }
        Ok(())
    }

    // Wallets

    pub fn get_wallets(&self) -> &Vec<Wallet> {
        self.wallets.get_all()
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

    pub fn get_active_wallet(&self) -> Option<&Wallet> {
        self.wallets.get_active()
    }

    pub fn change_wallet(&mut self, public_key: String) -> Result<(), CustomError> {
        self.wallets.set_active(&public_key)?;
        self.gui_sender.send(GUIEvents::WalletChanged)?;
        Ok(())
    }

    pub fn update_wallets(&mut self, block: &Block) -> Result<(), CustomError> {
        let wallets_updated = self.wallets.update(block, &self.utxo)?;
        if wallets_updated {
            self.gui_sender
                .send(GUIEvents::WalletsUpdated)
                .map_err(|_| CustomError::CannotInitGUI)?;
        }
        Ok(())
    }

    // UTXO

    pub fn get_active_wallet_balance(&self) -> Result<u64, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };
        self.utxo.wallet_balance(active_wallet)
    }

    pub fn get_active_wallet_utxo(
        &self,
    ) -> Result<Vec<(OutPoint, TransactionOutput)>, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };
        self.utxo.generate_wallet_utxo(active_wallet)
    }

    // Pending Tx

    pub fn update_pending_tx(&mut self, block: &Block) -> Result<(), CustomError> {
        self.pending_txs.update_pending_tx(block)
    }

    pub fn get_active_wallet_pending_txs(
        &self,
    ) -> Result<HashMap<OutPoint, TransactionOutput>, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };

        self.pending_txs.from_wallet(active_wallet)
    }

    pub fn append_pending_tx(&mut self, transaction: Transaction) -> Result<(), CustomError> {
        let updated = self.pending_txs.append_pending_tx(transaction);

        if updated {
            self.gui_sender
                .send(GUIEvents::NewPendingTx)
                .map_err(|_| CustomError::CannotInitGUI)?;
        }

        Ok(())
    }

    // Pending Blocks

    pub fn append_pending_block(&mut self, header_hash: Vec<u8>) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.append_block(header_hash)?;
        drop(pending_blocks);

        Ok(())
    }

    fn remove_pending_blocks(&self) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.drain();
        Ok(())
    }

    pub fn get_stale_requests(&self) -> Result<Vec<Vec<u8>>, CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.get_stale_requests()
    }

    pub fn is_block_pending(&self, block_hash: &Vec<u8>) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_block_pending(block_hash))
    }

    fn is_pending_blocks_empty(&self) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_empty())
    }

    // Transactions

    pub fn make_transaction(
        &mut self,
        mut outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<Transaction, CustomError> {
        let Some(active_wallet) = self.get_active_wallet() else { return Err(CustomError::WalletNotFound) };

        let total_value = self.calculate_total_value(fee, &outputs)?;
        let mut active_wallet_utxo = self.get_active_wallet_utxo()?;

        active_wallet_utxo.sort_by(|a, b| b.1.value.cmp(&a.1.value));
        let (inputs, total_input_value) = calculate_inputs(&active_wallet_utxo, total_value);

        let change = total_input_value - total_value;
        if change > 0 {
            outputs.insert(active_wallet.pubkey.clone(), change);
        }

        Transaction::create(active_wallet, inputs, outputs)
    }

    fn calculate_total_value(
        &self,
        fee: u64,
        outputs: &HashMap<String, u64>,
    ) -> Result<u64, CustomError> {
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

fn calculate_inputs(
    active_wallet_utxo: &[(OutPoint, TransactionOutput)],
    total_value: u64,
) -> (Vec<OutPoint>, u64) {
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
