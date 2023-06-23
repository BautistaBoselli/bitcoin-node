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

const TRANSFER_OUTPUTS: u8 = 3;

#[derive(Clone)]
pub struct GUITransfer {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUITransfer {
    pub fn handle_events(&mut self, message: &GUIActions) {
        let result = match message {
            GUIActions::WalletChanged => self.reset_tx_fields(),
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
        let logger_sender = self.logger_sender.clone();

        send_button.connect_clicked(move |_| {
            let mut outputs = HashMap::new();
            for i in 0..TRANSFER_OUTPUTS {
                match get_output(&builder, i, logger_sender.clone()) {
                    Ok(Some((pubkey, value))) => outputs.insert(pubkey, value),
                    Ok(None) => continue,
                    Err(error) => {
                        send_log(&logger_sender, Log::Error(error));
                        return;
                    }
                };
            }

            // let receiver_1 = get_output(&builder, 1, logger_sender.clone());
            // let receiver_2 = get_output(&builder, 2, logger_sender.clone());
            // let receiver_3 = get_output(&builder, 3, logger_sender.clone());

            // if let Ok(Some((pubkey, value))) = receiver_1 {
            //     outputs.insert(pubkey, value);
            // }
            // if let Ok(Some((pubkey, value))) = receiver_2 {
            //     outputs.insert(pubkey, value);
            // }
            // if let Ok(Some((pubkey, value))) = receiver_3 {
            //     outputs.insert(pubkey, value);
            // }

            let fee_entry: gtk::Entry = match get_gui_element(&builder, "tx-fee") {
                Ok(fee_entry) => fee_entry,
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };

            match fee_entry
                .text()
                .to_string()
                .parse::<u64>()
                .map_err(|_| CustomError::InvalidFee)
            {
                Ok(fee) => {
                    if fee <= 0 {
                        send_log(&logger_sender, Log::Error(CustomError::InvalidFee));
                        return;
                    }
                    if let Err(error) = node_action_sender_clone
                        .send(NodeAction::MakeTransaction((outputs, fee)))
                        .map_err(|_| CustomError::CannotSendMessageToChannel)
                    {
                        send_log(&logger_sender, Log::Error(error));
                        return;
                    };
                }
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };
        });
        Ok(())
    }

    fn reset_tx_fields(&self) -> Result<(), CustomError> {
        let fee_entry: gtk::Entry = get_gui_element(&self.builder, "tx-fee")?;
        fee_entry.set_text("0");

        for i in 0..TRANSFER_OUTPUTS {
            let receiver_pubkey: gtk::Entry =
                get_gui_element(&self.builder, &format!("output-{}-pubkey", i))?;
            receiver_pubkey.set_text("");
            let receiver_value: gtk::Entry =
                get_gui_element(&self.builder, &format!("output-{}-value", i))?;
            receiver_value.set_text("");
        }

        Ok(())
    }
}

fn get_output(
    builder: &gtk::Builder,
    i: u8,
    logger_sender: Sender<Log>,
) -> Result<Option<(String, u64)>, CustomError> {
    //let check: gtk::ToggleButton = get_gui_element(&builder, &format!("output-{}-check", i))?;

    let pubkey: gtk::Entry = get_gui_element(&builder, &format!("output-{}-pubkey", i))?;
    let value: gtk::Entry = get_gui_element(&builder, &format!("output-{}-value", i))?;

    if pubkey.text().to_string().is_empty() && value.text().to_string().is_empty() {
        return Ok(None);
    }
    if pubkey.text().to_string().len() != 34 || value.text().to_string().is_empty() {
        let message = Log::Error(CustomError::InvalidTransferFields);
        send_log(&logger_sender, message);
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
