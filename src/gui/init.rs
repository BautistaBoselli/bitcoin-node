use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    glib::{self, Receiver},
    prelude::BuilderExtManual,
    traits::{
        ButtonExt, ComboBoxExt, ComboBoxTextExt, DialogExt, EntryExt, LabelExt, MessageDialogExt,
        WidgetExt,
    },
};

use crate::{error::CustomError, node_state::NodeState};

pub enum GUIActions {
    Log(String),
    WalletChanged,
}

pub fn gui_init(
    gui_receiver: Receiver<GUIActions>,
    node_state_ref: Arc<Mutex<NodeState>>,
    logger_sender: mpsc::Sender<String>,
) -> Result<(), CustomError> {
    if gtk::init().is_err() {
        return Err(CustomError::CannotInitGUI);
    }

    let glade_src = include_str!("gui.glade");
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.object("window1").unwrap();
    let logs: gtk::Label = builder.object("logs").unwrap();
    let button: gtk::Button = builder.object("add-wallet-button").unwrap();
    let dialog: gtk::Dialog = builder.object("add-wallet-dialog").unwrap();
    let dialog_action: gtk::Button = builder.object("add-wallet-action").unwrap();
    let dialog_cancel: gtk::Button = builder.object("add-wallet-cancel").unwrap();
    let dialog_wallet_name: gtk::Entry = builder.object("add-wallet-name").unwrap();
    let dialog_wallet_pubkey: gtk::Entry = builder.object("add-wallet-pubkey").unwrap();
    let dialog_wallet_privkey: gtk::Entry = builder.object("add-wallet-privkey").unwrap();
    let select_wallet_cb: gtk::ComboBoxText = builder.object("select-wallet-combo-box").unwrap();
    let debug_button: gtk::Button = builder.object("debug").unwrap();
    let dialog_validation_error: gtk::MessageDialog = builder.object("validation-error").unwrap();
    let label_balance: gtk::Label = builder.object("label-balance").unwrap();

    update_wallet_combo_box(node_state_ref.clone(), select_wallet_cb.clone())?;

    //update balance

    //add wallet dialog
    let dialog_clone = dialog.clone();
    button.connect_clicked(move |_| {
        println!("Button clicked!");
        dialog_clone.run();
        dialog_clone.hide();
    });

    //select wallet comboboxtext
    let node_state_ref_clone = node_state_ref.clone();
    select_wallet_cb.connect_changed(move |cb| {
        switch_active_wallet(node_state_ref_clone.clone(), cb.clone());
    });

    //confirm button
    let dialog_clone = dialog.clone();
    let dialog_wallet_name_clone = dialog_wallet_name.clone();
    let dialog_wallet_pubkey_clone = dialog_wallet_pubkey.clone();
    let dialog_wallet_privkey_clone = dialog_wallet_privkey.clone();
    let node_state_ref_clone = node_state_ref.clone();
    dialog_action.connect_clicked(move |_| {
        let mut node_state = node_state_ref_clone.lock().unwrap();
        match node_state.append_wallet(
            dialog_wallet_name_clone.text().to_string(),
            dialog_wallet_pubkey_clone.text().to_string(),
            dialog_wallet_privkey_clone.text().to_string(),
        ) {
            Ok(_) => {}
            Err(e) => {
                if let CustomError::Validation(error) = e {
                    dialog_validation_error.set_secondary_text(Some(error.as_str()));
                } else {
                    dialog_validation_error.set_secondary_text(Some("unknown error"));
                }
                dialog_validation_error.run();
                dialog_validation_error.hide();
                return;
            }
        }
        drop(node_state);
        match update_wallet_combo_box(node_state_ref_clone.clone(), select_wallet_cb.clone()) {
            Ok(_) => {}
            Err(e) => {
                println!("Error actualizando combo box: {}", e);
            }
        }
        select_wallet_cb.set_active_id(Some(dialog_wallet_pubkey_clone.text().as_str()));
        switch_active_wallet(node_state_ref_clone.clone(), select_wallet_cb.clone());
        dialog_wallet_name_clone.set_text("");
        dialog_wallet_pubkey_clone.set_text("");
        dialog_wallet_privkey_clone.set_text("");
        dialog_clone.hide();
    });

    //cancel button
    let dialog_clone = dialog.clone();
    dialog_cancel.connect_clicked(move |_| {
        dialog_wallet_name.set_text("");
        dialog_wallet_pubkey.set_text("");
        dialog_wallet_privkey.set_text("");
        dialog_clone.hide();
    });

    //debug button
    let node_state_ref_clone = node_state_ref.clone();
    debug_button.connect_clicked(move |_| {
        let node_state = node_state_ref_clone.lock().unwrap();
        println!("active_wallet: {:?}", node_state.get_active_wallet());
        drop(node_state);
    });

    window.show_all();

    gui_receiver.attach(None, move |message| {
        match message {
            GUIActions::Log(message) => {
                logs.set_text(message.as_str());
            }

            GUIActions::WalletChanged => {
                let node_state = node_state_ref.lock().unwrap();
                match node_state.get_balance() {
                    Ok(balance) => {
                        let balance_btc = (balance as f64) / 100000000.0;
                        label_balance.set_text(
                            format!("Balance:    {} BTC", balance_btc.to_string()).as_str(),
                        );
                    }
                    Err(_) => logger_sender
                        .send(String::from("Error getting balance"))
                        .unwrap_or_else(|_| {
                            println!("Error sending log message");
                            println!("Error getting balance");
                        }),
                }

                drop(node_state);
            }
        }

        glib::Continue(true)
    });

    gtk::main();

    Ok(())
}

fn switch_active_wallet(
    node_state_ref: Arc<Mutex<NodeState>>,
    select_wallet_cb: gtk::ComboBoxText,
) {
    if let Some(active_pubkey) = select_wallet_cb.active_id() {
        let mut node_state = node_state_ref.lock().unwrap();
        node_state.change_wallet(active_pubkey.to_string());
        if let Some(active_wallet) = node_state.get_active_wallet() {
            select_wallet_cb.set_active_id(Some(active_wallet.pubkey.as_str()));
        } else {
            select_wallet_cb.set_active_id(None);
        }
        drop(node_state);
    }
}

fn update_wallet_combo_box(
    _node_state_ref: Arc<Mutex<NodeState>>,
    select_wallet_cb: gtk::ComboBoxText,
) -> Result<(), CustomError> {
    let node_state = _node_state_ref.lock()?;
    select_wallet_cb.remove_all();
    for wallet in node_state.get_wallets() {
        select_wallet_cb.append(Some(wallet.pubkey.as_str()), wallet.name.as_str());
    }
    drop(node_state);
    Ok(())
}
