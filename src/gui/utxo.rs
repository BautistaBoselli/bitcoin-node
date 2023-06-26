use std::sync::{mpsc::Sender, Arc, Mutex};

use gtk::{
    traits::{ButtonExt, ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
    structs::block_header::hash_as_string,
};

use super::init::{get_gui_element, GUIEvents};

#[derive(Clone)]
pub struct GUIUtxo {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIUtxo {
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
        let node_state = node_state_ref_clone
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;
        let wallet_utxo = node_state.get_active_wallet_utxo()?;
        reset_table(&utxo_list_box);

        for (out_point, tx_out) in wallet_utxo.iter() {
            let utxo_row = gtk::ListBoxRow::new();
            let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let tx_hash_label = gtk::Label::new(None);
            let value_string = format!("{:.8} BTC", (tx_out.value as f64) / 100_000_000.0);
            let value_label = gtk::Label::new(Some(&value_string.as_str()));
            let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let button = gtk::Button::new();

            button.set_label("Merkle Proof");

            tx_hash_label.set_expand(true);
            tx_hash_label.set_markup(
                format!("<small>{}</small>", hash_as_string(out_point.hash.clone())).as_str(),
            );
            value_label.set_width_request(128);
            button_box.set_width_request(128);

            utxo_box.add(&tx_hash_label);
            utxo_box.add(&value_label);
            button_box.add(&button);
            utxo_box.add(&button_box);

            utxo_row.add(&utxo_box);
            utxo_row.show_all();
            utxo_list_box.add(&utxo_row);
        }
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
    let value_label = gtk::Label::new(None);
    let action_label = gtk::Label::new(None);

    tx_hash_label.set_expand(true);
    tx_hash_label.set_markup("<b>Tx Hash</b>");

    value_label.set_width_request(128);
    value_label.set_markup("<b>Value</b>");

    action_label.set_width_request(128);
    action_label.set_markup("<b>Action</b>");

    utxo_box.add(&tx_hash_label);
    utxo_box.add(&value_label);
    utxo_box.add(&action_label);

    utxo_row.add(&utxo_box);
    utxo_row.show_all();
    list_box.add(&utxo_row);
}
