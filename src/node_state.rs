use std::{
    collections::HashMap,
    fs,
    net::SocketAddrV6,
    path::Path,
    sync::{mpsc, Arc, Mutex},
};

use gtk::glib::Sender;

use crate::{
    error::CustomError,
    gui::init::GUIEvents,
    logger::{send_log, Log},
    messages::{block::Block, get_headers::GetHeaders, headers::Headers, transaction::Transaction},
    peer::Peer,
    states::{
        headers_state::HeadersState,
        pending_blocks_state::PendingBlocks,
        pending_txs_state::PendingTxs,
        utxo_state::{UTXOValue, UTXO},
        wallets_state::Wallets,
    },
    structs::{block_header::BlockHeader, movement::Movement, outpoint::OutPoint},
    wallet::Wallet,
};

/// NodeState es una estructura que contiene el estado del nodo.
/// Los elementos son (para mas informacion de cada una de estas estructuras ver su documentacion en la carpeta states):
/// - logger_sender: Sender para enviar logs al logger.
/// - gui_sender: Sender para enviar eventos a la interfaz grafica.
/// - headers: HeadersState.
/// - wallets: Wallets.
/// - pending_blocks_ref: Referencia a PendingBlocks.
/// - utxo: UTXO.
/// - pending_txs: PendingTxs.
/// - blocks_sync: Indica si los bloques estan sincronizados.
/// - Peers: Vector de peers.
pub struct NodeState {
    store_path: String,
    logger_sender: mpsc::Sender<Log>,
    gui_sender: Sender<GUIEvents>,
    headers: HeadersState,
    peers: Vec<Peer>,
    wallets: Wallets,
    pending_blocks_ref: Arc<Mutex<PendingBlocks>>,
    utxo: UTXO,
    pending_txs: PendingTxs,
    blocks_sync: bool,
}

impl NodeState {
    /// Inicializa el estado del nodo. Inicializa todas sus estructuras indicando donde se encuentra el archivo donde se guardan.
    pub fn new(
        logger_sender: mpsc::Sender<Log>,
        gui_sender: Sender<GUIEvents>,
        store_path: &String,
    ) -> Result<Arc<Mutex<Self>>, CustomError> {
        send_log(
            &logger_sender,
            Log::Message(String::from("Initializing node state...")),
        );
        create_store_dir(store_path)?;

        let headers =
            HeadersState::new(format!("{}/headers.bin", store_path), logger_sender.clone())?;
        let pending_blocks_ref = PendingBlocks::new(&store_path, headers.get_all());

        let node_state_ref = Arc::new(Mutex::new(Self {
            store_path: store_path.clone(),
            logger_sender: logger_sender,
            gui_sender,
            headers,
            peers: vec![],
            wallets: Wallets::new(format!("{}/wallets.bin", store_path))?,
            pending_blocks_ref,
            utxo: UTXO::new(store_path.clone(), "/utxo.bin".to_string())?,
            pending_txs: PendingTxs::new(),
            blocks_sync: false,
        }));

        Ok(node_state_ref)
    }

    /// Agrega un bloque nuevo, lo guarda en su archivo y actualiza los pending_blocks, wallets, pending_txs y utxo.
    /// Tambien verifica si ahora el nodo esta actualizado con la red
    pub fn append_block(&mut self, block_hash: Vec<u8>, block: &Block) -> Result<(), CustomError> {
        let path = format!(
            "{}/blocks/{}.bin",
            self.store_path,
            block.header.hash_as_string()
        );
        block.save(path)?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove_block(&block_hash)?;
        drop(pending_blocks);

        self.verify_sync()?;

        self.update_wallets(block)?;
        self.update_pending_tx(block)?;

        if self.is_synced() {
            self.utxo.update_from_block(block, true)?;
        }

        Ok(())
    }

    pub fn get_block(&self, block_string_hash: String) -> Result<Block, CustomError> {
        let path = format!("{}/blocks/{}.bin", self.store_path, block_string_hash);
        Block::restore(path)
    }

    /********************     PEERS     ********************/

    /// devuelve referencia a los peers del nodo
    pub fn get_peers(&mut self) -> &mut Vec<Peer> {
        &mut self.peers
    }

    pub fn get_peer(&mut self, address: &SocketAddrV6) -> Option<&mut Peer> {
        self.peers.iter_mut().find(|p| &p.address == address)
    }

    /// agrega varios peers nuevos al nodo
    pub fn append_peers(&mut self, peers: Vec<Peer>) {
        self.peers.extend(peers);
    }

    pub fn remove_peer(&mut self, address: SocketAddrV6) {
        let index = self
            .peers
            .iter()
            .position(|p| p.address == address)
            .unwrap();

        self.peers.remove(index);
    }

    pub fn peer_send_headers(&mut self, address: SocketAddrV6) {
        let peer = self.peers.iter_mut().find(|p| p.address == address);
        if let Some(peer) = peer {
            peer.send_headers = true;
        }
    }

    pub fn peer_requested_headers(&mut self, address: SocketAddrV6) {
        let peer = self.peers.iter_mut().find(|p| p.address == address);
        if let Some(peer) = peer {
            peer.requested_headers = true;
        }
    }

    /********************     HEADERS     ********************/

    /// devuelve el hash del ultimo header guardado
    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.get_last_header_hash()
    }

    /// agrega un header nuevo en HeadersState
    pub fn append_headers(&mut self, headers: &Headers) -> Result<(), CustomError> {
        self.headers.append_headers(headers)?;
        self.gui_sender.send(GUIEvents::NewHeaders)?;
        self.verify_sync()
    }

    /// Devuelve los ultimos count headers del HeaderState
    pub fn get_last_headers(&self, count: usize) -> Vec<(usize, BlockHeader)> {
        self.headers.get_last_headers(count)
    }

    pub fn get_headers(&self, get_headers: GetHeaders) -> Vec<BlockHeader> {
        self.headers.get_headers(get_headers)
    }

    /********************     SYNC     ********************/

    /// Devuelve true si el nodo esta sincronizado con la red
    pub fn is_synced(&self) -> bool {
        self.headers.is_synced() && self.blocks_sync && self.utxo.is_synced()
    }

    /// Devuelve true si los bloques estan sincronizados
    pub fn is_blocks_sync(&self) -> bool {
        self.blocks_sync
    }

    /// Verifica si el nodo esta sincronizado con la red
    /// Si el nodo esta sincronizado, envia un evento a la interfaz grafica para indicar que el nodo esta listo para usarse
    /// Si el nodo no esta sincronizado, verifica si los headers estan sincronizados
    /// Si los headers estan sincronizados, verifica si los bloques estan sincronizados
    /// Si los bloques estan sincronizados, genera el UTXO
    ///
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

    /// Verifica si los bloques estan sincronizados
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

    /********************     WALLETS     ********************/

    /// Devuelve todas las wallets del nodo
    pub fn get_wallets(&self) -> &Vec<Wallet> {
        self.wallets.get_all()
    }

    /// Agrega una wallet nueva a WalletState
    pub fn append_wallet(
        &mut self,
        name: String,
        public_key: String,
        private_key: String,
    ) -> Result<(), CustomError> {
        let new_wallet = Wallet::new(name, public_key, private_key, &self.utxo)?;
        self.wallets.append(new_wallet)
    }

    /// Devuelve la wallet activa de WalletState
    pub fn get_active_wallet(&self) -> Option<&Wallet> {
        self.wallets.get_active()
    }

    /// Cambia la wallet activa de WalletState
    pub fn change_wallet(&mut self, public_key: String) -> Result<(), CustomError> {
        self.wallets.set_active(&public_key)?;
        self.gui_sender.send(GUIEvents::WalletChanged)?;
        Ok(())
    }

    /// Actualiza las wallets de WalletState
    pub fn update_wallets(&mut self, block: &Block) -> Result<(), CustomError> {
        let wallets_updated = self.wallets.update(block, &self.utxo)?;
        if wallets_updated {
            self.gui_sender
                .send(GUIEvents::WalletsUpdated)
                .map_err(|_| CustomError::CannotInitGUI)?;
        }
        Ok(())
    }

    /********************     UTXO     ********************/

    /// Devuelve el balance de la wallet activa
    pub fn get_active_wallet_balance(&self) -> Result<u64, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };
        self.utxo.wallet_balance(active_wallet)
    }

    /// Devuelve el UTXO de la wallet activa
    pub fn get_active_wallet_utxo(&self) -> Result<Vec<(OutPoint, UTXOValue)>, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };
        self.utxo.generate_wallet_utxo(active_wallet)
    }

    /********************     PENDING TXs     ********************/

    /// Actualiza las pending txs de PendingTxs
    pub fn update_pending_tx(&mut self, block: &Block) -> Result<(), CustomError> {
        self.pending_txs.update_pending_tx(block)
    }

    /// Devuelve las pending txs de la wallet activa
    pub fn get_active_wallet_pending_txs(&self) -> Result<Vec<Movement>, CustomError> {
        let Some(active_wallet) = self.wallets.get_active() else { return Err(CustomError::WalletNotFound) };

        self.pending_txs.from_wallet(active_wallet, &self.utxo)
    }

    /// Agrega una pending tx nueva a PendingTxs
    pub fn append_pending_tx(&mut self, transaction: Transaction) -> Result<bool, CustomError> {
        let updated = self.pending_txs.append_pending_tx(transaction);

        if updated {
            self.gui_sender
                .send(GUIEvents::NewPendingTx)
                .map_err(|_| CustomError::CannotInitGUI)?;
            send_log(
                &self.logger_sender,
                Log::Message("New pending transaction received".to_string()),
            );
        }

        Ok(updated)
    }

    pub fn get_pending_tx(&self, tx_hash: &Vec<u8>) -> Option<Transaction> {
        self.pending_txs.get_pending_tx(tx_hash)
    }

    /********************     PENDING BLOCKS     ********************/

    /// Agrega un pending block nuevo a PendingBlocks
    pub fn append_pending_block(&mut self, header_hash: Vec<u8>) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.append_block(header_hash)?;
        drop(pending_blocks);

        Ok(())
    }

    /// Remueve todos los pending blocks de PendingBlocks
    fn remove_pending_blocks(&self) -> Result<(), CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.drain();
        Ok(())
    }

    /// Devuelve los pending blocks de PendingBlocks
    pub fn get_stale_requests(&self) -> Result<Vec<Vec<u8>>, CustomError> {
        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.get_stale_requests()
    }

    /// Devuelve true si el bloque esta en PendingBlocks
    pub fn is_block_pending(&self, block_hash: &Vec<u8>) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_block_pending(block_hash))
    }

    /// Devuelve true si PendingBlocks esta vacio
    pub fn is_pending_blocks_empty(&self) -> Result<bool, CustomError> {
        let pending_blocks = self.pending_blocks_ref.lock()?;
        Ok(pending_blocks.is_empty())
    }

    /********************     TRANSACTIONS     ********************/

    /// Realiza una transaccion nueva para la active wallet de WalletsState
    /// con los outputs y el fee recibidos por parametro
    /// Devuelve la transaccion creada
    /// Si no hay una wallet activa, devuelve un error
    /// Si no hay suficientes fondos, devuelve un error
    pub fn make_transaction(
        &mut self,
        mut outputs: HashMap<String, u64>,
        fee: u64,
    ) -> Result<Transaction, CustomError> {
        let Some(active_wallet) = self.get_active_wallet() else { return Err(CustomError::WalletNotFound) };

        let total_value = self.calculate_total_value(fee, &outputs)?;
        let mut active_wallet_utxo = self.get_active_wallet_utxo()?;

        active_wallet_utxo.sort_by(|a, b| b.1.tx_out.value.cmp(&a.1.tx_out.value));
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
            return Err(CustomError::InsufficientFunds);
        }
        Ok(total_value)
    }
}

fn calculate_inputs(
    active_wallet_utxo: &[(OutPoint, UTXOValue)],
    total_value: u64,
) -> (Vec<OutPoint>, u64) {
    let mut inputs = vec![];
    let mut total_input_value = 0;
    for (out_point, tx_out) in active_wallet_utxo.iter() {
        inputs.push(out_point.clone());
        total_input_value += tx_out.tx_out.value;
        if total_input_value >= total_value {
            break;
        }
    }
    (inputs, total_input_value)
}

fn create_store_dir(path: &String) -> Result<(), CustomError> {
    let path = Path::new(path);
    if !path.exists() {
        fs::create_dir(path)?;
    }
    let blocks_path = path.join("blocks");
    if !blocks_path.exists() {
        fs::create_dir(blocks_path)?;
    }
    Ok(())
}
