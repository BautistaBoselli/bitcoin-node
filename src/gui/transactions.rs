use std::sync::{mpsc::Sender, Arc, Mutex};

use gtk::{
    traits::{ButtonExt, ContainerExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::{block::Block, headers::hash_as_string},
    node_state::NodeState,
};

use super::init::{get_gui_element, GUIActions};

#[derive(Clone)]
pub struct GUITransactions {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUITransactions {
    pub fn handle_events(&mut self, message: &GUIActions) {
        let result = match message {
            GUIActions::WalletChanged => self.update_txs(),
            GUIActions::NewBlock => self.update_txs(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn update_txs(&self) -> Result<(), CustomError> {
        let tx_list_box: gtk::ListBox = get_gui_element(&self.builder, "transactions-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock().unwrap();
        let history = node_state.get_active_wallet().unwrap().get_history();
        remove_transactions(&tx_list_box);
        for movement in history.iter().rev() {
            let tx_row = gtk::ListBoxRow::new();
            let tx_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let label = gtk::Label::new(Some(movement.value.to_string().as_str()));
            let button = gtk::Button::new();
            button.set_label("Merkle Proof");

            let movement_clone = movement.clone();
            button.connect_clicked(move |_| {
                println!(
                    "Block: {:?}, Tx: {:?}",
                    movement_clone.block_hash, movement_clone.tx_hash
                );

                match movement_clone.block_hash {
                    Some(ref block_hash) => {
                        let block = Block::restore(hash_as_string(block_hash.to_owned())).unwrap();
                        let (mp_flags, mp_hashes) = block
                            .generate_merkle_path(movement_clone.tx_hash.to_owned())
                            .unwrap();

                        println!("Merkle Flags: {:?}", mp_flags);
                        println!("Merkle Hashes: {:?}", mp_hashes);
                    }
                    None => {
                        println!("Tx not confirmed yet");
                    }
                }
            });

            tx_box.add(&label);
            tx_box.add(&button);

            tx_row.add(&tx_box);
            tx_row.show_all();
            tx_list_box.add(&tx_row);
        }
        drop(node_state);
        Ok(())
    }
}

fn remove_transactions(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
}
