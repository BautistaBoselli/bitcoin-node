use gtk::traits::{GtkWindowExt, WidgetExt};
use std::sync::mpsc;

use super::init::{get_gui_element, GUIEvents};
use crate::{
    error::CustomError,
    logger::{send_log, Log},
};

#[derive(Clone)]
pub struct GUIWindow {
    pub builder: gtk::Builder,
    pub logger_sender: mpsc::Sender<Log>,
}

impl GUIWindow {
    pub fn initialize(&self) -> Result<(), CustomError> {
        self.show_loading_window()?;
        Ok(())
    }

    fn show_loading_window(&self) -> Result<(), CustomError> {
        let load_window: gtk::Window = get_gui_element(&self.builder, "load-window")?;
        load_window.set_default_size(600, 400);
        load_window.set_resizable(false);
        load_window.show_all();
        Ok(())
    }

    pub fn handle_events(&self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::NodeStateReady => self.handle_node_state_ready(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    fn handle_node_state_ready(&self) -> Result<(), CustomError> {
        self.show_main_window()?;
        Ok(())
    }

    pub fn show_main_window(&self) -> Result<(), CustomError> {
        let load_window: gtk::Window = get_gui_element(&self.builder, "load-window")?;
        load_window.hide();
        let main_window: gtk::Window = get_gui_element(&self.builder, "main-window")?;
        main_window.show_all();
        Ok(())
    }
}
