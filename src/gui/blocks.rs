use std::sync::{mpsc::Sender, Arc, Mutex};

use gtk::{
    traits::{ContainerExt, LabelExt, WidgetExt},
    ListBox,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

use super::{
    init::{get_gui_element, GUIEvents},
    table_cells::{number_label, time_label, tx_hash_label},
};

#[derive(Clone)]
pub struct GUIBlocks {
    pub logger_sender: Sender<Log>,
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
}

impl GUIBlocks {
    pub fn handle_events(&mut self, message: &GUIEvents) {
        let result = match message {
            GUIEvents::NewHeaders => self.update_blocks(),
            _ => Ok(()),
        };

        if let Err(error) = result {
            send_log(&self.logger_sender, Log::Error(error));
        }
    }

    pub fn initialize(&self) -> Result<(), CustomError> {
        self.update_blocks()
    }

    fn update_blocks(&self) -> Result<(), CustomError> {
        let blocks_list_box: gtk::ListBox = get_gui_element(&self.builder, "blocks-list")?;
        let node_state_ref_clone = self.node_state_ref.clone();
        let node_state = node_state_ref_clone.lock()?;
        let headers = node_state.get_last_headers(100);

        reset_table(&blocks_list_box);
        for header in headers.iter() {
            let utxo_row = gtk::ListBoxRow::new();
            let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            utxo_box.set_margin_top(8);
            utxo_box.set_margin_bottom(8);

            utxo_box.add(&time_label(header.timestamp));
            utxo_box.add(&tx_hash_label(header.hash()));
            utxo_box.add(&number_label(header.version as i64));
            utxo_box.add(&number_label(header.bits as i64));

            utxo_row.add(&utxo_box);
            utxo_row.show_all();
            blocks_list_box.add(&utxo_row);
        }
        Ok(())
    }
}

fn reset_table(list_box: &ListBox) {
    list_box.foreach(|child| {
        list_box.remove(child);
    });
    let utxo_row = gtk::ListBoxRow::new();
    let utxo_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let tx_hash_label = gtk::Label::new(None);
    let time_label = gtk::Label::new(None);
    let nbits_label = gtk::Label::new(None);
    let version_label = gtk::Label::new(None);

    time_label.set_width_request(92);
    time_label.set_markup("<b>Time</b>");

    tx_hash_label.set_expand(true);
    tx_hash_label.set_markup("<b>Block Hash</b>");

    nbits_label.set_width_request(100);
    nbits_label.set_markup("<b>nBits</b>");

    version_label.set_width_request(100);
    version_label.set_markup("<b>Version</b>");

    utxo_box.add(&time_label);
    utxo_box.add(&tx_hash_label);
    utxo_box.add(&version_label);
    utxo_box.add(&nbits_label);

    utxo_row.add(&utxo_box);
    utxo_row.show_all();
    list_box.add(&utxo_row);
}