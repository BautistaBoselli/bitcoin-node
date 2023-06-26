use std::sync::{mpsc::Sender, Arc, Mutex};

use gtk::{
    traits::{ButtonExt, ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::block::Block,
    node_state::NodeState,
    structs::block_header::hash_as_string,
};

use super::init::{get_gui_element, GUIEvents};

#[derive(Clone)]
pub struct GUIHistory {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIHistory {
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
        let tx_list_box: gtk::ListBox = get_gui_element(&self.builder, "history-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;
        let active_wallet = match node_state.get_active_wallet() {
            Some(wallet) => wallet,
            None => {
                return Ok(());
            }
        };
        let history = active_wallet.get_history();
        reset_table(&tx_list_box);

        for movement in history.iter().rev() {
            let history_row = gtk::ListBoxRow::new();
            let history_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

            let tx_hash_label = gtk::Label::new(None);
            let side_label = gtk::Label::new(if movement.value > 0 {
                Some("Received")
            } else {
                Some("Sent")
            });
            let value_string = format!("{:.8} BTC", (movement.value as f64) / 100_000_000.0);
            let value_label = gtk::Label::new(Some(&value_string.as_str()));

            let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let button = gtk::Button::new();
            button.set_label("Merkle Proof");

            tx_hash_label.set_expand(true);
            tx_hash_label.set_markup(
                format!(
                    "<small>{}</small>",
                    hash_as_string(movement.tx_hash.clone())
                )
                .as_str(),
            );
            side_label.set_width_request(92);
            value_label.set_width_request(128);
            button_box.set_width_request(128);

            let movement_clone = movement.clone();
            let logger_sender = self.logger_sender.clone();
            button.connect_clicked(move |_| {
                println!(
                    "Block: {:?}, Tx: {:?}",
                    movement_clone.block_hash, movement_clone.tx_hash
                );

                match movement_clone.block_hash {
                    Some(ref block_hash) => {
                        let path =
                            format!("store/blocks/{}.bin", hash_as_string(block_hash.clone()));
                        let block = match Block::restore(path) {
                            Ok(block) => block,
                            Err(error) => {
                                send_log(&logger_sender, Log::Error(error));
                                return;
                            }
                        };
                        let (mp_flags, mp_hashes) =
                            match block.generate_merkle_path(movement_clone.tx_hash.to_owned()) {
                                Ok((mp_flags, mp_hashes)) => (mp_flags, mp_hashes),
                                Err(error) => {
                                    send_log(&logger_sender, Log::Error(error));
                                    return;
                                }
                            };
                        println!("Merkle Flags: {:?}", mp_flags);
                        println!("Merkle Hashes: {:?}", mp_hashes);
                    }
                    None => {
                        println!("Tx not confirmed yet");
                    }
                }
            });

            history_box.add(&tx_hash_label);
            history_box.add(&side_label);
            history_box.add(&value_label);
            button_box.add(&button);
            history_box.add(&button_box);

            history_row.add(&history_box);
            history_row.show_all();
            tx_list_box.add(&history_row);
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
