use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    traits::{ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

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
    pub available_balance: f64,
    pub pending_balance: f64,
}

impl GUIBalance {
    pub fn handle_events(&mut self, message: &GUIActions) {
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

    fn handle_wallet_changed(&mut self) -> Result<(), CustomError> {
        self.update_available_balance()?;
        self.update_pending_txs()
    }

    fn handle_new_block(&mut self) -> Result<(), CustomError> {
        self.update_available_balance()?;
        self.update_pending_txs()
    }

    fn handle_new_pending_tx(&mut self) -> Result<(), CustomError> {
        self.update_pending_txs()
    }

    fn update_available_balance(&mut self) -> Result<(), CustomError> {
        let node_state = self.node_state_ref.lock()?;

        match node_state.get_active_wallet_balance() {
            Ok(balance) => {
                self.available_balance = (balance as f64) / 100000000.0;
            }
            Err(error) => {
                send_log(&self.logger_sender, Log::Error(error));
            }
        }
        drop(node_state);

        self.update_balances()?;

        Ok(())
    }

    fn update_pending_txs(&mut self) -> Result<(), CustomError> {
        let pending_tx_list_box: gtk::ListBox =
            get_gui_element(&self.builder, "pending-transactions-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock().unwrap();
        let pending_transactions = node_state.get_active_wallet_pending_txs().unwrap();
        remove_transactions(&pending_tx_list_box);

        self.pending_balance = 0.0;
        for (_, tx_output) in pending_transactions {
            let pending_tx_row = gtk::ListBoxRow::new();
            pending_tx_row.add(&gtk::Label::new(Some(tx_output.value.to_string().as_str())));
            self.pending_balance = (tx_output.value as f64) / 100000000.0;
            pending_tx_row.show_all();
            pending_tx_list_box.add(&pending_tx_row);
        }
        drop(node_state);

        self.update_balances()?;

        Ok(())
    }

    fn update_balances(&self) -> Result<(), CustomError> {
        let available_balance: gtk::Label =
            get_gui_element(&self.builder, "label-available-balance")?;
        let pending_balance: gtk::Label = get_gui_element(&self.builder, "label-pending-balance")?;
        let total_balance: gtk::Label = get_gui_element(&self.builder, "label-total-balance")?;

        available_balance
            .set_text(format!("Balance:    {:.8} BTC", self.available_balance).as_str());
        pending_balance.set_text(format!("Pending:    {:.8} BTC", self.pending_balance).as_str());
        total_balance.set_text(
            format!(
                "Total:	     {:.8} BTC",
                self.available_balance + self.pending_balance
            )
            .as_str(),
        );

        Ok(())
    }
}

fn remove_transactions(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
}
