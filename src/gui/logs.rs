use std::sync::mpsc;

use chrono::Local;
use gtk::traits::{DialogExt, LabelExt, MessageDialogExt, WidgetExt};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
};

use super::init::{get_gui_element, GUIEvents};

#[derive(Clone)]
/// GUILogs es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con los logs. Muestra los logs en la pantalla de carga y en la pantalla principal.
/// Los elementos son:
/// - builder: Builder de gtk.
/// - logger_sender: Sender para enviar logs al logger.
pub struct GUILogs {
    pub builder: gtk::Builder,
    pub logger_sender: mpsc::Sender<Log>,
}

impl GUILogs {
    /// Maneja los GUIEvents recibidos y hace las acciones acorde a cada envento.
    /// Para Log: Actualiza los logs en la interfaz.
    pub fn handle_events(&self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::Log(log) => self.handle_log(log),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn handle_log(&self, log: &Log) -> Result<(), CustomError> {
        let logs: gtk::Label = get_gui_element(&self.builder, "logs")?;
        let load_screen_logs: gtk::Label = get_gui_element(&self.builder, "load-screen-logs")?;
        let dialog_error: gtk::MessageDialog = get_gui_element(&self.builder, "error-dialog")?;

        match log {
            Log::Message(string) => {
                let current_time = Local::now();
                let formatted_time = current_time.format("%Y-%m-%d %H:%M:%S");
                let formatted_string = format!("[{}] {}", formatted_time, string);
                logs.set_text(formatted_string.as_str());
                load_screen_logs.set_text(formatted_string.as_str());
            }
            Log::Error(error) => {
                dialog_error.set_text(Some("Error"));
                match error {
                    CustomError::Validation(ref explanation) => {
                        dialog_error.set_text(Some(error.description()));
                        dialog_error.set_secondary_text(Some(explanation.as_str()))
                    }
                    _ => dialog_error.set_secondary_text(Some(error.description())),
                }
                dialog_error.run();
                dialog_error.hide();
                dialog_error.set_text(Some(""));
                dialog_error.set_secondary_text(Some(""));
            }
        }

        Ok(())
    }
}
