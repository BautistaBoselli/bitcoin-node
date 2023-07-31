use std::sync::{mpsc, Arc, Mutex};

use gtk::traits::{ButtonExt, ComboBoxExt, ComboBoxTextExt, DialogExt, EntryExt, WidgetExt};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

use super::init::get_gui_element;

#[derive(Clone)]
/// GUIWallet es una estructura que contiene los elementos de la interfaz grafica
/// relacionados con la billetera. Permite agregar y cambiar de wallet y muestra la wallet activa.
pub struct GUIWallet {
    pub builder: gtk::Builder,
    pub node_state_ref: Arc<Mutex<NodeState>>,
    pub logger_sender: mpsc::Sender<Log>,
}

impl GUIWallet {
    /// Inicializa la los datos del combobox para seleccionar wallet.
    pub fn initialize(&self) -> Result<(), CustomError> {
        let select_wallet_cb: gtk::ComboBoxText =
            get_gui_element(&self.builder, "select-wallet-combo-box")?;

        update_wallet_combo_box(self.node_state_ref.clone(), select_wallet_cb)?;

        Ok(())
    }

    /// Agrega los callbacks a los elementos de la interfaz grafica.
    /// Los callbacks son:
    /// - handle_add_wallet_trigger: Muestra el dialogo para agregar una wallet.
    /// - handle_add_wallet_submit: Agrega la wallet ingresada a la lista de wallets.
    /// - cancel_add_wallet: Cancela el agregado de una wallet.
    /// - handle_change_wallet: Cambia la wallet activa.
    ///
    pub fn handle_interactivity(&self) -> Result<(), CustomError> {
        self.handle_add_wallet_trigger()?;
        self.handle_add_wallet_submit()?;
        self.cancel_add_wallet()?;
        self.handle_change_wallet()?;

        Ok(())
    }

    fn handle_change_wallet(&self) -> Result<(), CustomError> {
        let select_wallet_cb: gtk::ComboBoxText =
            get_gui_element(&self.builder, "select-wallet-combo-box")?;

        let node_state_ref = self.node_state_ref.clone();
        let builder = self.builder.clone();
        let logger_sender = self.logger_sender.clone();

        select_wallet_cb.connect_changed(move |_| {
            switch_active_wallet(&node_state_ref, &builder).unwrap_or_else(|error| {
                send_log(&logger_sender, Log::Error(error));
            });
        });

        Ok(())
    }

    fn handle_add_wallet_trigger(&self) -> Result<(), CustomError> {
        let trigger: gtk::Button = get_gui_element(&self.builder, "add-wallet-button")?;
        let dialog: gtk::Dialog = get_gui_element(&self.builder, "add-wallet-dialog")?;

        trigger.connect_clicked(move |_| {
            dialog.run();
            dialog.hide();
        });

        Ok(())
    }

    fn handle_add_wallet_submit(&self) -> Result<(), CustomError> {
        let dialog: gtk::Dialog = get_gui_element(&self.builder, "add-wallet-dialog")?;
        let action: gtk::Button = get_gui_element(&self.builder, "add-wallet-action")?;
        let name: gtk::Entry = get_gui_element(&self.builder, "add-wallet-name")?;
        let pubkey: gtk::Entry = get_gui_element(&self.builder, "add-wallet-pubkey")?;
        let privkey: gtk::Entry = get_gui_element(&self.builder, "add-wallet-privkey")?;
        let wallet_combobox: gtk::ComboBoxText =
            get_gui_element(&self.builder, "select-wallet-combo-box")?;
        let node_state_ref = self.node_state_ref.clone();
        let logger_sender = self.logger_sender.clone();

        action.connect_clicked(move |_| {
            let mut node_state = match node_state_ref
                .lock()
                .map_err(|_| CustomError::CannotLockGuard)
            {
                Ok(node_state) => node_state,
                Err(error) => {
                    send_log(&logger_sender, Log::Error(error));
                    return;
                }
            };
            if let Err(error) = node_state.append_wallet(
                name.text().to_string(),
                pubkey.text().to_string(),
                privkey.text().to_string(),
            ) {
                send_log(&logger_sender, Log::Error(error));
                drop(node_state);
                return;
            }
            drop(node_state);

            update_wallet_combo_box(node_state_ref.clone(), wallet_combobox.clone())
                .unwrap_or_else(|_| {
                    send_log(
                        &logger_sender,
                        Log::Message("Error updating combo box".to_string()),
                    )
                });
            name.set_text("");
            pubkey.set_text("");
            privkey.set_text("");
            dialog.hide();
        });

        Ok(())
    }

    fn cancel_add_wallet(&self) -> Result<(), CustomError> {
        let dialog: gtk::Dialog = get_gui_element(&self.builder, "add-wallet-dialog")?;
        let cancel: gtk::Button = get_gui_element(&self.builder, "add-wallet-cancel")?;
        let name: gtk::Entry = get_gui_element(&self.builder, "add-wallet-name")?;
        let pubkey: gtk::Entry = get_gui_element(&self.builder, "add-wallet-pubkey")?;
        let privkey: gtk::Entry = get_gui_element(&self.builder, "add-wallet-privkey")?;

        cancel.connect_clicked(move |_| {
            name.set_text("");
            pubkey.set_text("");
            privkey.set_text("");
            dialog.hide();
        });

        Ok(())
    }
}

fn switch_active_wallet(
    node_state_ref: &Arc<Mutex<NodeState>>,
    builder: &gtk::Builder,
) -> Result<(), CustomError> {
    let select_wallet_cb: gtk::ComboBoxText = get_gui_element(builder, "select-wallet-combo-box")?;

    if let Some(active_pubkey) = select_wallet_cb.active_id() {
        let mut node_state = node_state_ref.lock()?;
        node_state.change_wallet(active_pubkey.to_string())?;
        if let Some(active_wallet) = node_state.get_active_wallet() {
            select_wallet_cb.set_active_id(Some(active_wallet.pubkey.as_str()));
        } else {
            select_wallet_cb.set_active_id(None);
        }
        drop(node_state);
    }

    Ok(())
}

fn update_wallet_combo_box(
    node_state_ref: Arc<Mutex<NodeState>>,
    select_wallet_cb: gtk::ComboBoxText,
) -> Result<(), CustomError> {
    let node_state = node_state_ref.lock()?;
    select_wallet_cb.remove_all();
    for wallet in node_state.get_wallets() {
        select_wallet_cb.append(Some(wallet.pubkey.as_str()), wallet.name.as_str());
    }
    drop(node_state);
    Ok(())
}
