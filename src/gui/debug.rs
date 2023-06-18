use std::{sync::{Arc, Mutex}, collections::HashMap};

use gtk::{traits::ButtonExt};

use crate::{error::CustomError, node_state::NodeState};

use super::init::get_gui_element;

#[derive(Clone)]
pub struct GUIDebug {
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIDebug {
    pub fn handle_interactivity(&self) -> Result<(), CustomError> {
        let debug_button: gtk::Button = get_gui_element(&self.builder, "debug")?;
        let node_state_ref = self.node_state_ref.clone();

        debug_button.connect_clicked(move |_| {
            let node_state = node_state_ref.lock().unwrap();
            let mut outputs = HashMap::new();
            outputs.insert(
                String::from("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"),
                1000000,
            );
            outputs.insert(
                String::from("1A1zP1eP5QGefi2DMPTfTL5SLmDonROuch"),
                1000000,
            );
            node_state.make_transaction(outputs, 500000);
            drop(node_state);
        });

        Ok(())
    }
}
