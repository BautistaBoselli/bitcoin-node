use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    glib::{self, Object, Receiver},
    prelude::{BuilderExtManual, IsA},
    traits::WidgetExt,
};

use crate::{error::CustomError, logger::Log, node_state::NodeState};

use super::{balance::GUIBalance, debug::GUIDebug, logs::GUILogs, wallet::GUIWallet};

pub enum GUIActions {
    Log(Log),
    WalletChanged,
    NewPendingTx,
}

pub struct GUI {
    builder: gtk::Builder,
    wallet: GUIWallet,
    balance: GUIBalance,
    logs: GUILogs,
    debug: GUIDebug,
}

impl GUI {
    pub fn start(
        gui_receiver: Receiver<GUIActions>,
        node_state_ref: Arc<Mutex<NodeState>>,
        logger_sender: mpsc::Sender<Log>,
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
        };

        let logs = GUILogs {
            builder: builder.clone(),
            logger_sender,
        };

        let debug = GUIDebug {
            builder: builder.clone(),
            node_state_ref,
        };

        let gui = Self {
            builder,
            wallet,
            balance,
            logs,
            debug,
        };

        gui.handle_interactivity()?;
        gui.gui_actions_loop(gui_receiver)?;

        gtk::main();

        Ok(())
    }

    pub fn handle_interactivity(&self) -> Result<(), CustomError> {
        // initialize
        self.wallet.initialize()?;
        // self.show_loading_window()?;
        self.show_main_window()?;

        // interactivity
        self.wallet.handle_interactivity()?;
        self.debug.handle_interactivity()?;

        Ok(())
    }

    fn show_main_window(&self) -> Result<(), CustomError> {
        let window: gtk::Window = get_gui_element(&self.builder, "window1")?;
        window.show_all();
        Ok(())
    }

    fn gui_actions_loop(&self, gui_receiver: Receiver<GUIActions>) -> Result<(), CustomError> {
        let balance = self.balance.clone();
        let logs = self.logs.clone();

        gui_receiver.attach(None, move |message| {
            // if message == GUIActions::NodeStateReady {
            //     // balance.handle_events(&message);
            //     hide_loading_window(&message);
            //     show_main_window(&message);
            // }

            balance.handle_events(&message);
            logs.handle_events(&message);

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
