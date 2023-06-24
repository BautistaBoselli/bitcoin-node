use std::io::{Read, Write};

use crate::{wallet::Wallet, error::CustomError, node_state::open_new_file, parser::BufferParser, messages::block::Block};

use super::utxo_state::UTXO;

pub struct Wallets {
    pub wallets: Vec<Wallet>,
    pub active_pubkey: Option<String>,
    pub path: String,
}

impl Wallets {
    pub fn new(path: String) -> Result<Self, CustomError> {
        let mut wallets = Self{
            wallets: Vec::new(),
            active_pubkey: None,
            path,
        };
        wallets.restore()?;
        Ok(wallets)
    }

    fn restore(&mut self) -> Result<(), CustomError> {
        let mut file = open_new_file(self.path.clone(), false)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;
        let mut parser = BufferParser::new(buffer);

        let mut wallets = vec![];
        while !parser.is_empty() {
            let wallet = Wallet::parse(&mut parser)?;
            wallets.push(wallet);
        }

        self.wallets = wallets;
        Ok(())
    }

    fn save(&self) -> Result<(), CustomError> {
        let mut file = open_new_file(self.path.clone(), false)?;

        let mut buffer = vec![];
        for wallet in &self.wallets {
            buffer.append(&mut wallet.serialize());
        }

        file.write_all(&buffer)?;
        Ok(())
    }

    pub fn set_active(&mut self, public_key: String) -> Result<(), CustomError> {
        self.active_pubkey = self
            .wallets
            .iter()
            .find(|wallet| wallet.pubkey == public_key)
            .map(|wallet| wallet.pubkey.clone());
        Ok(())
    }

    pub fn get_all(&self) -> &Vec<Wallet> {
        &self.wallets
    }

    pub fn append(
        &mut self,
        new_wallet: Wallet,
    ) -> Result<(), CustomError> {
        if self
            .wallets
            .iter()
            .any(|wallet| wallet.pubkey == new_wallet.pubkey)
        {
            return Err(CustomError::Validation(
                "Public key already exists".to_string(),
            ));
        }
        self.wallets.push(new_wallet);
        self.save()?;
        Ok(())
    }

    pub fn get_active(&self) -> Option<&Wallet> {
        match self.active_pubkey {
            Some(ref active_wallet) => self
                .wallets
                .iter()
                .find(|wallet| wallet.pubkey == *active_wallet),
            None => None,
        }
    }

    pub fn update(&mut self, block: &Block, utxo: &UTXO) -> Result<bool, CustomError> {
        let mut wallets_updated = false;
        for tx in block.transactions.iter() {
            for wallet in &mut self.wallets {
                let movement = tx.get_movement(&wallet.get_pubkey_hash()?, utxo)?;
                if let Some(mut movement) = movement {
                    movement.block_hash = Some(block.header.hash());
                    wallet.update_history(movement);
                    wallets_updated = true;
                }
            }
        };
        if wallets_updated {
            self.save()?;
        }
        Ok(wallets_updated && self.active_pubkey.is_some())
    }
}