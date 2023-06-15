use std::sync::{mpsc, Arc, Mutex};

use gtk::{
    glib::{self, Receiver},
    prelude::BuilderExtManual,
    traits::{
        ButtonExt, ComboBoxExt, ComboBoxTextExt, DialogExt, EntryExt, LabelExt, MessageDialogExt,
        WidgetExt, ContainerExt
    },
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

pub enum GUIActions {
    Log(Log),
    WalletChanged,
}

pub fn gui_init(
    gui_receiver: Receiver<GUIActions>,
    node_state_ref: Arc<Mutex<NodeState>>,
    logger_sender: mpsc::Sender<Log>,
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
    let dialog_error: gtk::MessageDialog = builder.object("error-dialog").unwrap();
    let label_balance: gtk::Label = builder.object("label-balance").unwrap();
    let pending_tx_list_box: gtk::ListBox = builder.object("pending-transactions-list").unwrap();

    update_wallet_combo_box(node_state_ref.clone(), select_wallet_cb.clone())?;

    //Pending transactions (provisorio)
    {
        let node_state_ref_clone = node_state_ref.clone();
        let node_state = node_state_ref_clone.lock().unwrap();
        let pending_transactions = node_state.get_pending_tx_from_wallet().unwrap();
        for (_, tx_output) in pending_transactions{
            let pending_tx_row = gtk::ListBoxRow::new();
            pending_tx_row.add(&gtk::Label::new(Some(tx_output.value.to_string().as_str())));
            pending_tx_list_box.add(&pending_tx_row);
        }
        drop(node_state);
    }


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
    let loggger_sender_clone = logger_sender.clone();
    dialog_action.connect_clicked(move |_| {
        let mut node_state = node_state_ref_clone.lock().unwrap();
        match node_state.append_wallet(
            dialog_wallet_name_clone.text().to_string(),
            dialog_wallet_pubkey_clone.text().to_string(),
            dialog_wallet_privkey_clone.text().to_string(),
        ) {
            Ok(_) => {}
            Err(error) => {
                send_log(&loggger_sender_clone, Log::Error(error));
                // if let CustomError::Validation(error) = e {
                //     dialog_error.set_secondary_text(Some(error.as_str()));
                // } else {
                //     dialog_error.set_secondary_text(Some("unknown error"));
                // }
                // dialog_error.set_text(Some("Incorrect values"));
                // dialog_error.run();
                // dialog_error.hide();
                // dialog_error.set_text(Some(""));
                // dialog_error.set_secondary_text(Some(""));

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
    let dialog_clone = dialog;
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
            GUIActions::Log(message) => match message {
                Log::Message(string) => {
                    logs.set_text(string.as_str());
                }
                Log::Error(error) => {
                    dialog_error.set_text(Some("Error"));
                    match error {
                        CustomError::Validation(ref explanation) => {
                            dialog_error.set_text(Some(error.description()));
                            dialog_error.set_secondary_text(Some(&explanation.as_str()))
                        }
                        _ => dialog_error.set_secondary_text(Some(error.description())),
                    }
                    dialog_error.run();
                    dialog_error.hide();
                    dialog_error.set_text(Some(""));
                    dialog_error.set_secondary_text(Some(""));
                }
            },

            GUIActions::WalletChanged => {
                let node_state = node_state_ref.lock().unwrap();
                match node_state.get_balance() {
                    Ok(balance) => {
                        let balance_btc = (balance as f64) / 100000000.0;
                        label_balance.set_text(format!("Balance:    {} BTC", balance_btc).as_str());
                    }
                    Err(error) => {
                        send_log(&logger_sender, Log::Error(error));
                    }
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
