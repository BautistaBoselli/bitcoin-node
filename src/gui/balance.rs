use std::sync::{mpsc, Arc, Mutex};

use gtk::traits::{ContainerExt, LabelExt, WidgetExt};

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
        let result = match message {
            GUIActions::WalletChanged => self.handle_wallet_changed(),
            GUIActions::NewPendingTx => self.handle_new_pending_tx(),
            GUIActions::NewBlock => self.handle_new_block(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn handle_wallet_changed(&self) -> Result<(), CustomError> {
        self.update_available_balance()?;
        self.update_pending_txs()?;
        self.update_txs()
    }

    fn handle_new_block(&self) -> Result<(), CustomError> {
        self.update_txs()
    }

    fn handle_new_pending_tx(&self) -> Result<(), CustomError> {
        self.update_available_balance()?;
        self.update_pending_txs()
    }

    fn update_available_balance(&self) -> Result<(), CustomError> {
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

    fn update_pending_txs(&self) -> Result<(), CustomError> {
        let pending_tx_list_box: gtk::ListBox =
            get_gui_element(&self.builder, "pending-transactions-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock().unwrap();
        let pending_transactions = node_state.get_pending_tx_from_wallet().unwrap();
        println!("POR MOSTRAR {} transacciones", pending_transactions.len());
        pending_tx_list_box.foreach(|child| {
            pending_tx_list_box.remove(child);
        });
        for (_, tx_output) in pending_transactions {
            println!("tx: {:?}", tx_output);
            let pending_tx_row = gtk::ListBoxRow::new();
            pending_tx_row.add(&gtk::Label::new(Some(tx_output.value.to_string().as_str())));
            pending_tx_row.show_all();
            pending_tx_list_box.add(&pending_tx_row);
        }
        drop(node_state);

        Ok(())
    }

    fn update_txs(&self) -> Result<(), CustomError> {
        let tx_list_box: gtk::ListBox = get_gui_element(&self.builder, "transactions-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock().unwrap();
        let history = node_state.get_active_wallet().unwrap().get_history();
        println!("POR MOSTRAR {} transacciones", history.len());
        tx_list_box.foreach(|child| {
            tx_list_box.remove(child);
        });
        for movement in history {
            let tx_row = gtk::ListBoxRow::new();
            tx_row.add(&gtk::Label::new(Some(movement.value.to_string().as_str())));
            tx_row.show_all();
            tx_list_box.add(&tx_row);
        }
        drop(node_state);
        Ok(())
    }
}
