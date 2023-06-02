use std::sync::{Arc, Mutex};

use gtk::{
    glib::{self, Receiver},
    prelude::BuilderExtManual,
    traits::{LabelExt, WidgetExt},
};

use crate::{error::CustomError, messages::headers::BlockHeader, node_state::NodeState};

pub enum GUIActions {
    Log(String),
    Headers(Vec<BlockHeader>),
}

pub fn gui_init(
    gui_receiver: Receiver<GUIActions>,
    _node_state_ref: Arc<Mutex<NodeState>>,
) -> Result<(), CustomError> {
    if gtk::init().is_err() {
        return Err(CustomError::CannotInitGUI);
    }

    let glade_src = include_str!("gui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.object("window1").unwrap();
    let label: gtk::Label = builder.object("label1").unwrap();
    let blocks: gtk::Label = builder.object("blocks").unwrap();

    window.show_all();

    gui_receiver.attach(None, move |message| {
        match message {
            GUIActions::Log(message) => {
                label.set_text(message.as_str());
            }
            GUIActions::Headers(headers) => {
                blocks.set_text(headers.len().to_string().as_str());
            }
        }

        glib::Continue(true)
    });

    gtk::main();

    Ok(())
}
