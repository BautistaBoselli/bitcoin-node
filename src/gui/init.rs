use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    glib::{self, Object, Receiver},
    prelude::{BuilderExtManual, IsA},
};

use crate::{error::CustomError, logger::Log, node_state::NodeState, peer::NodeAction};

use super::{
    balance::GUIBalance, blocks::GUIBlocks, history::GUIHistory, logs::GUILogs,
    transfer::GUITransfer, utxo::GUIUtxo, wallet::GUIWallet, window::GUIWindow,
};

/// GUIEvents es un enum que contiene los eventos que se pueden recibir en el canal de eventos de la interfaz grafica.
/// Los eventos son:
/// - Log: Recibe un Log y lo muestra en la lista de logs.
/// - WalletChanged: Se cambio la wallet activa.
/// - WalletsUpdated: Se Actualizo alguna de las wallets cargadas.
/// - NewPendingTx: Alguna de las wallets cargadas recibio una pending transaction.
/// - NodeStateReady: El node state ya se sincronizo y se puede mostrar la informacion.
/// - NewBlock: Llego un nuevo bloque.
/// - TransactionSent: Se envio una transaccion del usuario.
/// - NewHeaders: Hay nuevos Headers.
pub enum GUIEvents {
    Log(Log),
    WalletChanged,
    WalletsUpdated,
    NewPendingTx,
    NodeStateReady,
    NewBlock,
    TransactionSent,
    NewHeaders,
}

/// GUI es una estructura que contiene los elementos que manejan la interfaz grafica
/// Contiene y les maneja el ciclo de vida a cada uno de los elementos de la interfaz grafica.
/// Los elementos son:
/// - node_action_sender: Sender para enviar acciones al nodo.
/// - wallet: GUIWallet.
/// - balance: GUIBalance.
/// - logs: GUILogs.
/// - history: GUIHistory.
/// - utxo: GUIUtxo.
/// - blocks: GUIBlocks.
/// - transfer: GUITransfer.
/// - window: GUIWindow.
pub struct GUI {
    node_action_sender: mpsc::Sender<NodeAction>,
    wallet: GUIWallet,
    balance: GUIBalance,
    logs: GUILogs,
    history: GUIHistory,
    utxo: GUIUtxo,
    blocks: GUIBlocks,
    transfer: GUITransfer,
    window: GUIWindow,
}

impl GUI {
    /// Inicializa la interfaz grafica.
    /// Crea los elementos de la interfaz grafica y los inicializa.
    /// Inicializa el ciclo de vida de la interfaz grafica (escuchar los GUIEvents).
    pub fn start(
        gui_receiver: Receiver<GUIEvents>,
        node_state_ref: Arc<Mutex<NodeState>>,
        logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> Result<(), CustomError> {
        if gtk::init().is_err() {
            return Err(CustomError::CannotInitGUI);
        }

        let glade_src = include_str!("gui.glade");
        let builder = gtk::Builder::from_string(glade_src);

        let wallet = GUIWallet {
            builder: builder.clone(),
            node_state_ref: node_state_ref.clone(),
            logger_sender: logger_sender.clone(),
        };

        let balance = GUIBalance {
            builder: builder.clone(),
            node_state_ref: node_state_ref.clone(),
            logger_sender: logger_sender.clone(),
            available_balance: 0.0,
            pending_balance: 0.0,
        };

        let logs = GUILogs {
            builder: builder.clone(),
            logger_sender: logger_sender.clone(),
        };

        let history = GUIHistory {
            builder: builder.clone(),
            logger_sender: logger_sender.clone(),
            node_state_ref: node_state_ref.clone(),
        };

        let utxo = GUIUtxo {
            builder: builder.clone(),
            logger_sender: logger_sender.clone(),
            node_state_ref: node_state_ref.clone(),
        };

        let blocks = GUIBlocks {
            builder: builder.clone(),
            logger_sender: logger_sender.clone(),
            node_state_ref: node_state_ref.clone(),
        };

        let transfer = GUITransfer {
            builder: builder.clone(),
            logger_sender: logger_sender.clone(),
            node_state_ref,
        };

        let window = GUIWindow {
            builder,
            logger_sender,
        };

        let gui = Self {
            node_action_sender,
            wallet,
            balance,
            logs,
            history,
            utxo,
            blocks,
            transfer,
            window,
        };

        gui.handle_interactivity()?;
        gui.gui_actions_loop(gui_receiver)?;

        gtk::main();

        Ok(())
    }

    /// Inicializa los elementos de la interfaz grafica.
    /// Establece la interactividad de los elementos que la necesitan.
    pub fn handle_interactivity(&self) -> Result<(), CustomError> {
        // initialize
        self.wallet.initialize()?;
        self.window.initialize()?;
        self.blocks.initialize()?;

        // interactivity
        self.wallet.handle_interactivity()?;
        self.transfer
            .handle_interactivity(&self.node_action_sender)?;

        Ok(())
    }

    fn gui_actions_loop(&self, gui_receiver: Receiver<GUIEvents>) -> Result<(), CustomError> {
        let mut balance = self.balance.clone();
        let logs = self.logs.clone();
        let mut transactions = self.history.clone();
        let window = self.window.clone();
        let mut transfer = self.transfer.clone();
        let mut utxo = self.utxo.clone();
        let mut blocks = self.blocks.clone();

        gui_receiver.attach(None, move |message| {
            balance.handle_events(&message);
            logs.handle_events(&message);
            transactions.handle_events(&message);
            window.handle_events(&message);
            transfer.handle_events(&message);
            utxo.handle_events(&message);
            blocks.handle_events(&message);

            glib::Continue(true)
        });

        Ok(())
    }
}

/// Devuelve un elemento de la interfaz grafica.
/// Si no existe el elemento devuelve un error.
pub fn get_gui_element<T: IsA<Object>>(
    builder: &gtk::Builder,
    name: &str,
) -> Result<T, CustomError> {
    let element: T = builder.object(name).ok_or(CustomError::MissingGUIElement)?;
    Ok(element)
}
