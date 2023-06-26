use std::sync::{mpsc::Sender, Arc, Mutex, MutexGuard};

use gtk::{
    traits::{ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
    states::utxo_state::UTXOValue,
    structs::outpoint::OutPoint,
};

use super::{
    init::{get_gui_element, GUIEvents},
    table_cells::{merkle_proof_button, time_label, tx_hash_label, value_label},
};

#[derive(Clone)]
/// GUIUtxo es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con los UTXO de una wallet y los lista (tx hash, fecha de creacion, valor y pedir el merkle proof de esa tx).
/// Los elementos son:
/// - builder: Builder de gtk.
/// - node_state_ref: Referencia al estado del nodo.
/// - logger_sender: Sender para enviar logs al logger.
pub struct GUIUtxo {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIUtxo {
    /// Maneja los GUIEvents recibidos y hace las acciones acorde a cada envento.
    /// Para WalletChanged: Actualiza la lista de UTXO.
    /// Para WalletsUpdated: Actualiza la lista de UTXO.
    pub fn handle_events(&mut self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::WalletChanged => self.update_utxo(),
            GUIEvents::WalletsUpdated => self.update_utxo(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn update_utxo(&self) -> Result<(), CustomError> {
        let utxo_list_box: gtk::ListBox = get_gui_element(&self.builder, "utxo-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock()?;
        let wallet_utxo = get_wallet_sorted_utxo(node_state)?;

        reset_table(&utxo_list_box);
        for (out_point, utxo_value) in wallet_utxo.iter() {
            let utxo_row = gtk::ListBoxRow::new();
            let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

            utxo_box.add(&tx_hash_label(out_point.hash.clone()));
            utxo_box.add(&time_label(utxo_value.block_timestamp));
            utxo_box.add(&value_label(utxo_value.tx_out.value as i64));
            utxo_box.add(&merkle_proof_button(
                Some(utxo_value.block_hash.clone()),
                out_point.hash.clone(),
                self.logger_sender.clone(),
            ));

            utxo_row.add(&utxo_box);
            utxo_row.show_all();
            utxo_list_box.add(&utxo_row);
        }
        Ok(())
    }
}

fn get_wallet_sorted_utxo(
    node_state: MutexGuard<'_, NodeState>,
) -> Result<Vec<(OutPoint, UTXOValue)>, CustomError> {
    let mut wallet_utxo = node_state.get_active_wallet_utxo()?;
    wallet_utxo.sort_by(|a, b| {
        if a.1.block_timestamp > b.1.block_timestamp {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    Ok(wallet_utxo)
}

fn reset_table(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
    let utxo_row = gtk::ListBoxRow::new();
    let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let tx_hash_label = gtk::Label::new(None);
    let time_label = gtk::Label::new(None);
    let value_label = gtk::Label::new(None);
    let action_label = gtk::Label::new(None);

    tx_hash_label.set_expand(true);
    tx_hash_label.set_markup("<b>Tx Hash</b>");

    time_label.set_width_request(92);
    time_label.set_markup("<b>Time</b>");

    value_label.set_width_request(128);
    value_label.set_markup("<b>Value</b>");

    action_label.set_width_request(128);
    action_label.set_markup("<b>Action</b>");

    utxo_box.add(&tx_hash_label);
    utxo_box.add(&time_label);
    utxo_box.add(&value_label);
    utxo_box.add(&action_label);

    utxo_row.add(&utxo_box);
    utxo_row.show_all();
    list_box.add(&utxo_row);
}
