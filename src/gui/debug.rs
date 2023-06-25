use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Arc, Mutex},
};

use gtk::traits::ButtonExt;

use crate::{error::CustomError, node_state::NodeState, peer::NodeAction};

use super::init::get_gui_element;

#[derive(Clone)]
pub struct GUIDebug {
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIDebug {
    pub fn handle_interactivity(
        &self,
        node_action_sender: &Sender<NodeAction>,
    ) -> Result<(), CustomError> {
        let debug_button: gtk::Button = get_gui_element(&self.builder, "debug")?;

        let clone = node_action_sender.clone();
        debug_button.connect_clicked(move |_| {
            let mut outputs = HashMap::new();
            outputs.insert(
                String::from("mniwvWuHto1y9vmMEqQX5mvrXMVYDizbu2"),
                1_000_000,
            );
            //outputs.insert(String::from("1A1zP1eP5QGefi2DMPTfTL5SLmDonROuch"), 700000);

            let fee = 500_000;

            clone
                .send(NodeAction::MakeTransaction((outputs, fee)))
                .unwrap();
        });

        Ok(())
    }
}
