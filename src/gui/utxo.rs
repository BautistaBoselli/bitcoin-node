use std::sync::{mpsc::Sender, Arc, Mutex};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
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
        let mut wallet_utxo = node_state.get_active_wallet_utxo()?;
        reset_table(&utxo_list_box);

        wallet_utxo.sort_by(|a, b| {
            if a.1.block_timestamp > b.1.block_timestamp {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        });
        for (out_point, utxo_value) in wallet_utxo.iter() {
            let utxo_row = gtk::ListBoxRow::new();
            let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let tx_hash_label = gtk::Label::new(None);

            let time_label = gtk::Label::new(None);
            let current_time = Local::now();
            let formatted_time = current_time.format("%m/%d").to_string();
            if let Some(datetime) =
                NaiveDateTime::from_timestamp_millis(utxo_value.block_timestamp as i64 * 1000)
            {
                let tx_time = DateTime::<Local>::from_utc(datetime, *Local::now().offset());
                let formatted_tx_time = tx_time.format("%m/%d").to_string();
                if formatted_tx_time == formatted_time {
                    time_label.set_text(tx_time.format("%H:%M").to_string().as_str());
                } else {
                    time_label.set_text(&formatted_tx_time);
                }
            }

            let value_string = format!(
                "{:.8} BTC",
                (utxo_value.tx_out.value as f64) / 100_000_000.0
            );
            let value_label = gtk::Label::new(Some(&value_string.as_str()));
            let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let button = gtk::Button::new();

            button.set_label("Merkle Proof");
            let out_point_clone = out_point.clone();
            let utxo_value_clone = utxo_value.clone();
            let logger_sender = self.logger_sender.clone();
            button.connect_clicked(move |_| {
                let path = format!(
                    "store/blocks/{}.bin",
                    hash_as_string(utxo_value_clone.block_hash.clone())
                );
                let block = match Block::restore(path) {
                    Ok(block) => block,
                    Err(error) => {
                        send_log(&logger_sender, Log::Error(error));
                        return;
                    }
                };
                let (mp_flags, mp_hashes) =
                    match block.generate_merkle_path(out_point_clone.hash.to_owned()) {
                        Ok((mp_flags, mp_hashes)) => (mp_flags, mp_hashes),
                        Err(error) => {
                            send_log(&logger_sender, Log::Error(error));
                            return;
                        }
                    };
                println!("Merkle Flags: {:?}", mp_flags);
                println!("Merkle Hashes: {:?}", mp_hashes);
            });

            tx_hash_label.set_expand(true);
            tx_hash_label.set_markup(
                format!("<small>{}</small>", hash_as_string(out_point.hash.clone())).as_str(),
            );
            time_label.set_width_request(92);
            value_label.set_width_request(128);
            button_box.set_width_request(128);

            utxo_box.add(&tx_hash_label);
            utxo_box.add(&time_label);
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
