use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    glib::{self, Object, Receiver},
    prelude::{BuilderExtManual, IsA},
};

use crate::{error::CustomError, logger::Log, node_state::NodeState, peer::NodeAction};

use super::{
    balance::GUIBalance, debug::GUIDebug, logs::GUILogs, transactions::GUITransactions,
    transfer::GUITransfer, wallet::GUIWallet, window::GUIWindow,
};

pub enum GUIEvents {
    Log(Log),
    WalletChanged,
    WalletsUpdated,
    NewPendingTx,
    NodeStateReady,
    NewBlock,
}

pub struct GUI {
    node_action_sender: mpsc::Sender<NodeAction>,
    wallet: GUIWallet,
    balance: GUIBalance,
    logs: GUILogs,
    debug: GUIDebug,
    transactions: GUITransactions,
    transfer: GUITransfer,
    window: GUIWindow,
}

impl GUI {
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

        let debug = GUIDebug {
            builder: builder.clone(),
            node_state_ref: node_state_ref.clone(),
        };

        let transactions = GUITransactions {
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
            debug,
            transactions,
            transfer,
            window,
        };

        gui.handle_interactivity()?;
        gui.gui_actions_loop(gui_receiver)?;

        gtk::main();

        Ok(())
    }

    pub fn handle_interactivity(&self) -> Result<(), CustomError> {
        // initialize
        self.wallet.initialize()?;
        self.window.initialize()?;

        // interactivity
        self.wallet.handle_interactivity()?;
        self.debug.handle_interactivity(&self.node_action_sender)?;
        self.transfer
            .handle_interactivity(&self.node_action_sender)?;

        Ok(())
    }

    fn gui_actions_loop(&self, gui_receiver: Receiver<GUIEvents>) -> Result<(), CustomError> {
        let mut balance = self.balance.clone();
        let logs = self.logs.clone();
        let mut transactions = self.transactions.clone();
        let window = self.window.clone();
        let mut transfer = self.transfer.clone();

        gui_receiver.attach(None, move |message| {
            balance.handle_events(&message);
            logs.handle_events(&message);
            transactions.handle_events(&message);
            window.handle_events(&message);
            transfer.handle_events(&message);

            glib::Continue(true)
        });

        Ok(())
    }
}

pub fn get_gui_element<T: IsA<Object>>(
    builder: &gtk::Builder,
    name: &str,
) -> Result<T, CustomError> {
    let element: T = builder.object(name).ok_or(CustomError::MissingGUIElement)?;
    Ok(element)
}
