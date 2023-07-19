use std::sync::{mpsc::Sender, Arc, Mutex};

use gtk::{
    traits::{ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

use super::{
    init::{get_gui_element, GUIEvents},
    table_cells::{merkle_proof_button, side_label, tx_hash_label, value_label},
};

#[derive(Clone)]
/// GUIHistory es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con el historial de movimientos de una wallet y los lista (tx hash, enviado o recibido, valor y pedir el merkle proof de esa tx).
/// Los elementos son:
/// - builder: Builder de gtk.
/// - node_state_ref: Referencia al estado del nodo.
/// - logger_sender: Sender para enviar logs al logger.
pub struct GUIHistory {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIHistory {
    /// Maneja los GUIEvents recibidos y hace las acciones acorde a cada envento.
    /// Para WalletChanged: Actualiza la lista de movimientos.
    /// Para WalletsUpdated: Actualiza la lista de movimientos.
    pub fn handle_events(&mut self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::WalletChanged => self.update_txs(),
            GUIEvents::WalletsUpdated => self.update_txs(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn update_txs(&self) -> Result<(), CustomError> {
        let history_list_box: gtk::ListBox = get_gui_element(&self.builder, "history-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock()?;
        let Some(active_wallet) = node_state.get_active_wallet() else { return Ok(()) };
        let history = active_wallet.get_history();
        reset_table(&history_list_box);

        for movement in history.iter().rev() {
            let history_row = gtk::ListBoxRow::new();
            let history_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

            history_box.add(&tx_hash_label(movement.tx_hash.clone()));
            history_box.add(&side_label(movement.value));
            history_box.add(&value_label(movement.value));
            history_box.add(&merkle_proof_button(
                movement.block_hash.clone(),
                movement.tx_hash.clone(),
                self.logger_sender.clone(),
                self.node_state_ref.clone(),
            ));

            history_row.add(&history_box);
            history_row.show_all();
            history_list_box.add(&history_row);
        }
        drop(node_state);
        Ok(())
    }
}

fn reset_table(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
    let utxo_row = gtk::ListBoxRow::new();
    let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let tx_hash_label = gtk::Label::new(None);
    let side_label = gtk::Label::new(None);
    let value_label = gtk::Label::new(None);
    let action_label = gtk::Label::new(None);

    tx_hash_label.set_expand(true);
    tx_hash_label.set_markup("<b>Tx Hash</b>");

    side_label.set_width_request(92);
    side_label.set_markup("<b>Side</b>");

    value_label.set_width_request(128);
    value_label.set_markup("<b>Value</b>");

    action_label.set_width_request(128);
    action_label.set_markup("<b>Action</b>");

    utxo_box.add(&tx_hash_label);
    utxo_box.add(&side_label);
    utxo_box.add(&value_label);
    utxo_box.add(&action_label);

    utxo_row.add(&utxo_box);
    utxo_row.show_all();
    list_box.add(&utxo_row);
}
