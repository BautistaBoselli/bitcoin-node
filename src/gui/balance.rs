use std::sync::{mpsc, Arc, Mutex};

use gtk::traits::LabelExt;

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

use super::init::{get_gui_element, GUIActions};

#[derive(Clone)]
pub struct GUIBalance {
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
    pub logger_sender: mpsc::Sender<Log>,
}

impl GUIBalance {
    pub fn handle_events(&self, message: &GUIActions) {
        let result: Result<(), CustomError> = match message {
            GUIActions::WalletChanged => self.handle_wallet_changed(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn handle_wallet_changed(&self) -> Result<(), CustomError> {
        let label_balance: gtk::Label = get_gui_element(&self.builder, "label-balance")?;
        let node_state = self.node_state_ref.lock()?;

        match node_state.get_balance() {
            Ok(balance) => {
                let balance_btc = (balance as f64) / 100000000.0;
                label_balance.set_text(format!("Balance:    {} BTC", balance_btc).as_str());
            }
            Err(error) => {
                send_log(&self.logger_sender, Log::Error(error));
            }
        }
        drop(node_state);
        Ok(())
    }
}
