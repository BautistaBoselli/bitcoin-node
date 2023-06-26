use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
};

use gtk::traits::{ButtonExt, DialogExt, EntryExt, LabelExt, WidgetExt};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
    peer::NodeAction,
};

use super::init::{get_gui_element, GUIEvents};

const TRANSFER_OUTPUTS: u8 = 3;

#[derive(Clone)]
/// GUITransfer es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con el envio de transacciones. Permite enviar transacciones a una o mas direcciones ingresando para cada una la pubkey y el monto, ademas del fee.
/// Los elementos son:
/// - builder: Builder de gtk.
/// - node_state_ref: Referencia al estado del nodo.
/// - logger_sender: Sender para enviar logs al logger.
pub struct GUITransfer {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUITransfer {
    /// Maneja los GUIEvents recibidos y hace las acciones acorde a cada envento.
    /// Para WalletChanged: Resetea los campos de la transaccion.
    /// Para TransactionSent: Muestra un dialogo de transaccion enviada y resetea los campos.
    pub fn handle_events(&mut self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::WalletChanged => self.reset_tx_fields(),
            GUIEvents::TransactionSent => self.handle_sent_transaction(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }
    /// Establece los callbacks de los elementos de la interfaz grafica.
    /// Para el boton de enviar transaccion: Envia la transaccion al nodo (o abre una ventana de error en caso de estar mal ingresada) con los valores leidos de la interfaz.
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
                match get_output(&builder, i) {
                    Ok(Some((pubkey, value))) => outputs.insert(pubkey, value),
                    Ok(None) => continue,
                    Err(error) => {
                        send_log(&logger_sender, Log::Error(error));
                        return;
                    }
                };
            }

            let fee_entry: gtk::Entry = match get_gui_element(&builder, "tx-fee") {
                Ok(fee_entry) => fee_entry,
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };

            match fee_entry.text().to_string().parse::<u64>() {
                Ok(fee) => {
                    if fee == 0 {
                        send_log(&logger_sender, Log::Error(CustomError::InvalidFee));
                        return;
                    }
                    if node_action_sender_clone
                        .send(NodeAction::MakeTransaction((outputs, fee)))
                        .is_err()
                    {
                        send_log(
                            &logger_sender,
                            Log::Error(CustomError::CannotSendMessageToChannel),
                        );
                    };
                }
                Err(_) => {
                    send_log(&logger_sender, Log::Error(CustomError::InvalidFee));
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
            let label: gtk::Label =
                get_gui_element(&self.builder, &format!("tx-information-label{}", i))?;
            label.set_text("");
        }
        Ok(())
    }

    fn handle_sent_transaction(&self) -> Result<(), CustomError> {
        let dialog: gtk::MessageDialog = get_gui_element(&self.builder, "successful-tx-dialog")?;

        for i in 0..TRANSFER_OUTPUTS {
            let label: gtk::Label =
                get_gui_element(&self.builder, &format!("tx-information-label{}", i))?;
            if let Ok(Some((pubkey, value))) = get_output(&self.builder, i) {
                label.set_text(&format!("Transaction of {} sent to: {}", value, pubkey));
            };
        }
        dialog.run();
        dialog.hide();

        self.reset_tx_fields()?;
        Ok(())
    }
}

fn get_output(builder: &gtk::Builder, i: u8) -> Result<Option<(String, u64)>, CustomError> {
    let pubkey: gtk::Entry = get_gui_element(builder, &format!("output-{}-pubkey", i))?;
    let value: gtk::Entry = get_gui_element(builder, &format!("output-{}-value", i))?;

    if pubkey.text().to_string().is_empty() && value.text().to_string().is_empty() {
        return Ok(None);
    }
    if pubkey.text().to_string().len() != 34 || value.text().to_string().is_empty() {
        return Err(CustomError::InvalidTransferFields);
    }

    let value = value
        .text()
        .to_string()
        .parse::<u64>()
        .map_err(|_| CustomError::InvalidValue)?;

    Ok(Some((pubkey.text().to_string(), value)))
}
