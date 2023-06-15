use std::sync::mpsc;

use gtk::traits::{DialogExt, LabelExt, MessageDialogExt, WidgetExt};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
};

use super::init::{get_gui_element, GUIActions};

#[derive(Clone)]
pub struct GUILogs {
    pub builder: gtk::Builder,
    pub logger_sender: mpsc::Sender<Log>,
}

impl GUILogs {
    pub fn handle_events(&self, message: &GUIActions) {
        let result = match message {
            GUIActions::Log(log) => self.handle_log(log),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    pub fn handle_log(&self, log: &Log) -> Result<(), CustomError> {
        let logs: gtk::Label = get_gui_element(&self.builder, "logs")?;
        let dialog_error: gtk::MessageDialog = get_gui_element(&self.builder, "error-dialog")?;

        match log {
            Log::Message(string) => {
                logs.set_text(string.as_str());
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
