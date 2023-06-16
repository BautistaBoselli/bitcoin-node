use std::sync::{Arc, Mutex};

use gtk::traits::ButtonExt;

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
            println!(
                "active_wallet: {:?}",
                node_state.get_pending_tx_from_wallet()
            );
            drop(node_state);
        });

        Ok(())
    }
}
