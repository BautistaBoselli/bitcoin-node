use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    traits::{BoxExt, ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

use super::{
    init::{get_gui_element, GUIEvents},
    table_cells::{side_label, value_label},
};

#[derive(Clone)]
/// GUIBalance es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con el balance de la billetera y transacciones pendientes.
///
/// Los elementos son:
/// - builder: Builder de gtk.
/// - node_state_ref: Referencia al estado del nodo.
/// - logger_sender: Sender para enviar logs al logger.
/// - available_balance: Balance disponible de la billetera.
/// - pending_balance: Balance pendiente de la billetera.
pub struct GUIBalance {
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
    pub logger_sender: mpsc::Sender<Log>,
    pub available_balance: f64,
    pub pending_balance: f64,
}

impl GUIBalance {
    /// Maneja los GUIEvents recibidos y hace las acciones acorde a cada envento.
    ///
    /// Para WalletChanged: Actualiza el balance pendiente y disponible y las transacciones pendientes.
    /// Para WalletsUpdated: Actualiza el balance pendiente y disponible y las transacciones pendientes.
    /// Para NewPendingTx: Actualiza las transacciones pendientes y el balance pendinente.
    pub fn handle_events(&mut self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::WalletChanged => self.handle_wallet_changed(),
            GUIEvents::NewPendingTx => self.handle_new_pending_tx(),
            GUIEvents::WalletsUpdated => self.handle_wallet_updated(),
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

    fn handle_wallet_updated(&mut self) -> Result<(), CustomError> {
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
                self.available_balance = balance as f64;
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
        let node_state = node_state_ref_clone.lock()?;
        if node_state.get_active_wallet().is_none() {
            return Ok(());
        }
        let pending_transactions = node_state.get_active_wallet_pending_txs()?;

        self.pending_balance = 0.0;
        reset_table(&pending_tx_list_box);
        for movement in pending_transactions {
            self.pending_balance += movement.value as f64;
            let pending_tx_row = gtk::ListBoxRow::new();
            let pending_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            pending_box.set_homogeneous(true);

            let side_label = side_label(movement.value);
            pending_box.add(&side_label);
            let value_label = value_label(movement.value);
            pending_box.add(&value_label);

            pending_tx_row.add(&pending_box);
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
        let transfer_balance: gtk::Label =
            get_gui_element(&self.builder, "label-transfer-balance")?;

        let available_btc = self.available_balance / 100_000_000.0;
        available_balance.set_text(format!("Balance:    {:.8} BTC", available_btc).as_str());

        let pending_btc = self.pending_balance / 100_000_000.0;
        pending_balance.set_text(format!("Pending:    {:.8} BTC", pending_btc).as_str());

        let total_satoshi = self.available_balance + self.pending_balance;
        let total_btc = total_satoshi / 100_000_000.0;
        let total_balance_string = format!("Total:	     {:.8} BTC", total_btc);
        let total_balance_string_satoshi = format!("Total:  {:.0} Sat", total_satoshi);

        total_balance.set_text(total_balance_string.as_str());
        transfer_balance.set_text(total_balance_string_satoshi.as_str());

        Ok(())
    }
}

fn reset_table(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
    let utxo_row = gtk::ListBoxRow::new();
    let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let side_label = gtk::Label::new(None);
    let value_label = gtk::Label::new(None);

    utxo_box.set_homogeneous(true);
    side_label.set_markup("<b>Side</b>");
    value_label.set_markup("<b>Value</b>");

    utxo_box.add(&side_label);
    utxo_box.add(&value_label);

    utxo_row.add(&utxo_box);
    utxo_row.show_all();
    list_box.add(&utxo_row);
}
