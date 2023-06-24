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
            GUIActions::WalletChanged => self.update(),
            GUIActions::WalletsUpdated => self.update(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn update(&self) -> Result<(), CustomError> {
        self.update_txs()?;
        self.update_utxo()?;
        Ok(())
    }

    fn update_txs(&self) -> Result<(), CustomError> {
        let tx_list_box: gtk::ListBox = get_gui_element(&self.builder, "movements-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;
        let active_wallet = match node_state.get_active_wallet() {
            Some(wallet) => wallet,
            None => return Err(CustomError::WalletNotFound),
        };
        let history = active_wallet.get_history();
        remove_transactions(&tx_list_box);

        for movement in history.iter().rev() {
            let tx_row = gtk::ListBoxRow::new();
            let tx_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let label = gtk::Label::new(Some(movement.value.to_string().as_str()));
            let button = gtk::Button::new();
            button.set_label("Merkle Proof");

            let movement_clone = movement.clone();
            let logger_sender = self.logger_sender.clone();
            button.connect_clicked(move |_| {
                println!(
                    "Block: {:?}, Tx: {:?}",
                    movement_clone.block_hash, movement_clone.tx_hash
                );

                match movement_clone.block_hash {
                    Some(ref block_hash) => {
                        let path = format!("store/blocks/{}.bin", hash_as_string(block_hash.clone()));
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

            tx_box.add(&label);
            tx_box.add(&button);

            tx_row.add(&tx_box);
            tx_row.show_all();
            tx_list_box.add(&tx_row);
        }
        drop(node_state);
        Ok(())
    }

    fn update_utxo(&self) -> Result<(), CustomError> {
        let utxo_list_box: gtk::ListBox = get_gui_element(&self.builder, "utxo-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;
        let wallet_utxo = node_state.get_active_wallet_utxo()?;
        remove_transactions(&utxo_list_box);
        for (_out_point, tx_out) in wallet_utxo.iter() {
            let utxo_row = gtk::ListBoxRow::new();
            let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let label = gtk::Label::new(Some(tx_out.value.to_string().as_str()));

            utxo_box.add(&label);

            utxo_row.add(&utxo_box);
            utxo_row.show_all();
            utxo_list_box.add(&utxo_row);
        }
        Ok(())
    }
}

fn remove_transactions(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
}
