use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
};

use gtk::traits::{ButtonExt, EntryExt};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
    peer::NodeAction,
};

use super::init::{get_gui_element, GUIActions};

#[derive(Clone)]
pub struct GUITransfer {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUITransfer {
    pub fn handle_events(&mut self, message: &GUIActions) {
        let result = match message {
            // GUIActions::WalletChanged => self.update_txs(),
            // GUIActions:: => self.update_txs(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }
    pub fn handle_interactivity(
        &self,
        node_action_sender: &Sender<NodeAction>,
    ) -> Result<(), CustomError> {
        let send_button: gtk::Button = get_gui_element(&self.builder, "send-tx")?;

        let node_action_sender_clone = node_action_sender.clone();
        let builder = self.builder.clone();

        send_button.connect_clicked(move |_| {
            let mut outputs = HashMap::new();
            let receiver_1 = get_output(&builder, 1);
            let receiver_2 = get_output(&builder, 2);
            let receiver_3 = get_output(&builder, 3);

            if let Ok(Some((pubkey, value))) = receiver_1 {
                outputs.insert(pubkey, value);
            }
            if let Ok(Some((pubkey, value))) = receiver_2 {
                outputs.insert(pubkey, value);
            }
            if let Ok(Some((pubkey, value))) = receiver_3 {
                outputs.insert(pubkey, value);
            }

            let fee_entry: gtk::Entry = get_gui_element(&builder, "tx-fee").unwrap();
            let fee = fee_entry.text().to_string().parse::<u64>().unwrap();

            node_action_sender_clone
                .send(NodeAction::MakeTransaction((outputs, fee)))
                .unwrap();
        });

        Ok(())
    }
}

fn get_output(builder: &gtk::Builder, i: u8) -> Result<Option<(String, u64)>, CustomError> {
    // let check: gtk::ToggleButton = get_gui_element(&builder, &format!("output-{}-check", i))?;

    let pubkey: gtk::Entry = get_gui_element(&builder, &format!("output-{}-pubkey", i))?;
    let value: gtk::Entry = get_gui_element(&builder, &format!("output-{}-value", i))?;

    if pubkey.text().to_string().is_empty() || value.text().to_string().is_empty() {
        return Ok(None);
    }

    Ok(Some((
        pubkey.text().to_string(),
        value
            .text()
            .parse::<u64>()
            .map_err(|_| CustomError::InvalidValue)?,
    )))
}
