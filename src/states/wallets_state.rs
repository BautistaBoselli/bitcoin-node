use std::io::{Read, Write};

use crate::{
    error::CustomError, messages::block::Block, node_state::open_new_file, parser::BufferParser,
    wallet::Wallet,
};

use super::utxo_state::UTXO;

pub struct Wallets {
    pub wallets: Vec<Wallet>,
    pub active_pubkey: Option<String>,
    pub path: String,
}

impl Wallets {
    pub fn new(path: String) -> Result<Self, CustomError> {
        let mut wallets = Self {
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

    pub fn append(&mut self, new_wallet: Wallet) -> Result<(), CustomError> {
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
        }
        if wallets_updated {
            self.save()?;
        }
        Ok(wallets_updated && self.active_pubkey.is_some())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, remove_file};

    use crate::messages::{
        headers::BlockHeader,
        transaction::{OutPoint, Transaction, TransactionInput, TransactionOutput},
    };

    use super::*;

    #[test]
    fn create_wallets_empty() {
        let wallets = Wallets::new("tests/wallets_empty.bin".to_string()).unwrap();
        assert_eq!(wallets.wallets.len(), 0);
        assert_eq!(wallets.active_pubkey, None);

        remove_file("tests/wallets_empty.bin".to_string()).unwrap();
    }

    #[test]
    fn create_wallets_restoring_a_wallet() {
        let wallets = Wallets::new("tests/test_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets.wallets.len(), 1);
        assert_eq!(wallets.active_pubkey, None);
    }

    #[test]
    fn append_wallet() {
        fs::copy(
            "tests/test_wallets.bin".to_string(),
            "tests/test_wallets_append.bin".to_string(),
        )
        .unwrap();

        let mut wallets = Wallets::new("tests/test_wallets_append.bin".to_string()).unwrap();
        assert_eq!(wallets.wallets.len(), 1);

        let new_wallet = Wallet::new(
            String::from("wallet 2"),
            String::from("mxz3drZtkg4R3u1RDL7zRPLsizvhmGWfr3"),
            String::from("private key 2"),
            &UTXO::new(String::from("tests/test_utxo.bin")).unwrap(),
        )
        .unwrap();

        wallets.append(new_wallet).unwrap();
        assert_eq!(wallets.wallets.len(), 2);

        remove_file("tests/test_wallets_append.bin".to_string()).unwrap();
    }

    #[test]
    fn append_wallet_duplicated_wallet() {
        fs::copy(
            "tests/test_wallets.bin".to_string(),
            "tests/test_wallets_append_duplicated.bin".to_string(),
        )
        .unwrap();

        let mut wallets =
            Wallets::new("tests/test_wallets_append_duplicated.bin".to_string()).unwrap();
        assert_eq!(wallets.wallets.len(), 1);

        let new_wallet = Wallet::new(
            String::from("wallet 2"),
            String::from("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm"),
            String::from("private key 2"),
            &UTXO::new(String::from("tests/test_utxo.bin")).unwrap(),
        )
        .unwrap();

        let result = wallets.append(new_wallet);
        assert!(result.is_err());

        remove_file("tests/test_wallets_append_duplicated.bin".to_string()).unwrap();
    }

    #[test]
    fn save_wallets() {
        let mut wallets = Wallets::new("tests/save_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets.wallets.len(), 0);

        let new_wallet = Wallet::new(
            String::from("wallet 2"),
            String::from("mxz3drZtkg4R3u1RDL7zRPLsizvhmGWfr3"),
            String::from("private key 2"),
            &UTXO::new(String::from("tests/test_utxo.bin")).unwrap(),
        )
        .unwrap();

        wallets.append(new_wallet).unwrap();
        assert_eq!(wallets.wallets.len(), 1);

        let wallets2 = Wallets::new("tests/save_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets2.wallets.len(), 1);

        remove_file("tests/save_wallets.bin".to_string()).unwrap();
    }

    #[test]
    fn get_wallets() {
        let wallets = Wallets::new("tests/test_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets.active_pubkey, None);

        let all_wallets = wallets.get_all();

        assert_eq!(all_wallets.len(), 1);
        assert_eq!(all_wallets[0].name, "wallet 1");
        assert_eq!(all_wallets[0].pubkey, "mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm");
    }

    #[test]
    fn set_active_wallet() {
        let mut wallets = Wallets::new("tests/test_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets.active_pubkey, None);

        wallets
            .set_active("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm".to_string())
            .unwrap();
        assert_eq!(
            wallets.active_pubkey,
            Some("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm".to_string())
        );
    }

    #[test]
    fn get_active_wallet() {
        let mut wallets = Wallets::new("tests/test_wallets.bin".to_string()).unwrap();
        assert_eq!(wallets.active_pubkey, None);

        assert!(wallets.get_active().is_none());

        wallets
            .set_active("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm".to_string())
            .unwrap();
        assert_eq!(
            wallets.active_pubkey,
            Some("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm".to_string())
        );

        let active_wallet = wallets.get_active().unwrap();
        assert_eq!(active_wallet.name, "wallet 1");
        assert_eq!(active_wallet.pubkey, "mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm");
    }

    #[test]
    fn update_wallets_from_new_block() {
        fs::copy(
            "tests/test_wallets.bin".to_string(),
            "tests/test_wallets_update.bin".to_string(),
        )
        .unwrap();

        let mut wallets = Wallets::new("tests/test_wallets_update.bin".to_string()).unwrap();
        assert_eq!(wallets.active_pubkey, None);

        wallets
            .set_active("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm".to_string())
            .unwrap();

        assert_eq!(wallets.get_active().unwrap().history.len(), 0);

        let block = Block {
            header: BlockHeader {
                version: 536887296,
                prev_block_hash: vec![],
                merkle_root: vec![],
                timestamp: 1686626483,
                bits: 421617023,
                nonce: 3878826733,
            },
            transactions: vec![Transaction {
                version: 1,
                inputs: vec![TransactionInput {
                    previous_output: OutPoint {
                        hash: vec![],
                        index: 4294967295,
                    },
                    script_sig: vec![],
                    sequence: 4294967295,
                }],
                outputs: vec![TransactionOutput {
                    value: 2366975,
                    script_pubkey: vec![
                        118, 169, 20, 27, 40, 219, 33, 69, 20, 4, 108, 105, 234, 87, 71, 50, 50,
                        154, 22, 16, 220, 64, 85, 136, 172,
                    ],
                }],
                lock_time: 0,
            }],
        };

        let utxo = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();

        let updated = wallets.update(&block, &utxo).unwrap();

        assert_eq!(updated, true);
        assert_eq!(wallets.get_active().unwrap().history.len(), 1);

        remove_file("tests/test_wallets_update.bin".to_string()).unwrap();
    }
}
