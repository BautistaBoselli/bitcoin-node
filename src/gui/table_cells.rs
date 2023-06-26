use std::sync::mpsc::Sender;

use chrono::{DateTime, Local, NaiveDateTime};
use gtk::traits::{ButtonExt, ContainerExt, LabelExt, WidgetExt};

use crate::{
    logger::{send_log, Log},
    messages::block::Block,
    structs::block_header::hash_as_string,
};

/// Genera un label formateado para un hash en formato hexadecimal y lo devuelve.
pub fn tx_hash_label(mut tx_hash: Vec<u8>) -> gtk::Label {
    let tx_hash_label = gtk::Label::new(None);

    tx_hash.reverse();
    let mut tx_hash_string = hash_as_string(tx_hash.clone());
    tx_hash_string.make_ascii_lowercase();

    tx_hash_label.set_text(tx_hash_string.as_str());

    tx_hash_label.set_expand(true);

    tx_hash_label
}

/// Genera un label formateado para una fecha y lo devuelve.
/// Si la fecha es de hoy, muestra la hora, sino muestra la fecha.
pub fn time_label(timestamp: u32) -> gtk::Label {
    let time_label = gtk::Label::new(None);
    let current_time = Local::now();
    let formatted_time = current_time.format("%m/%d").to_string();
    if let Some(datetime) = NaiveDateTime::from_timestamp_millis(timestamp as i64 * 1000) {
        let tx_time = DateTime::<Local>::from_utc(datetime, *Local::now().offset());
        let formatted_tx_time = tx_time.format("%m/%d").to_string();
        if formatted_tx_time == formatted_time {
            time_label.set_text(tx_time.format("%H:%M").to_string().as_str());
        } else {
            time_label.set_text(&formatted_tx_time);
        }
    }

    time_label.set_width_request(92);
    time_label
}

/// Genera un label formateado para un valor en satoshis y lo devuelve.
/// El valor se muestra en BTC.
pub fn value_label(value: i64) -> gtk::Label {
    let value_string = format!("{:.8} BTC", (value as f64) / 100_000_000.0);
    let value_label = gtk::Label::new(Some(value_string.as_str()));

    value_label.set_width_request(128);

    value_label
}

/// Genera un boton para pedir el merkle proof de una transaccion y lo devuelve.
/// Si el bloque no esta en la base de datos, no se muestra el boton.
pub fn merkle_proof_button(
    block_hash: Option<Vec<u8>>,
    tx_hash: Vec<u8>,
    logger_sender: Sender<Log>,
) -> gtk::Box {
    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

    if let Some(block_hash) = block_hash {
        let button = gtk::Button::new();

        button.set_label("Merkle Proof");
        let block_hash_string = hash_as_string(block_hash);
        button.connect_clicked(move |_| {
            let path = format!("store/blocks/{}.bin", block_hash_string);
            let block = match Block::restore(path) {
                Ok(block) => block,
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };
            let (mp_flags, mp_hashes) = match block.generate_merkle_path(tx_hash.to_vec()) {
                Ok((mp_flags, mp_hashes)) => (mp_flags, mp_hashes),
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };
            println!("Merkle Flags: {:?}", mp_flags);
            println!("Merkle Hashes: {:?}", mp_hashes);
        });

        button_box.add(&button);
    }
    button_box.set_width_request(128);

    button_box
}

/// Genera un label formateado que indica si se recibe o se envia en la transaccion.
/// Si el valor es positivo, se recibe, sino se envia
pub fn side_label(value: i64) -> gtk::Label {
    let side_label = gtk::Label::new(if value > 0 {
        Some("Received")
    } else {
        Some("Sent")
    });

    side_label.set_width_request(92);

    side_label
}

/// Genera un label formateado para un numero y lo devuelve.
pub fn number_label(value: i64) -> gtk::Label {
    let number_label = gtk::Label::new(Some(value.to_string().as_str()));

    number_label.set_width_request(100);

    number_label
}
