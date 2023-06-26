use std::{
    collections::{hash_map, HashMap},
    vec,
};

use crate::{
    error::CustomError,
    messages::{block::Block, transaction::Transaction},
    structs::movement::Movement,
    wallet::Wallet,
};

use super::utxo_state::UTXO;

pub struct PendingTxs {
    tx_set: HashMap<Vec<u8>, Transaction>,
}

impl Default for PendingTxs {
    fn default() -> Self {
        PendingTxs::new()
    }
}

impl PendingTxs {
    pub fn new() -> Self {
        PendingTxs {
            tx_set: HashMap::new(),
        }
    }

    pub fn append_pending_tx(&mut self, transaction: Transaction) -> bool {
        let tx_hash = transaction.hash();

        if let hash_map::Entry::Vacant(e) = self.tx_set.entry(tx_hash) {
            e.insert(transaction);
            return true;
        }
        false
    }

    pub fn update_pending_tx(&mut self, block: &Block) -> Result<(), CustomError> {
        for tx in &block.transactions {
            if self.tx_set.contains_key(&tx.hash()) {
                self.tx_set.remove(&tx.hash());
            }
        }

        Ok(())
    }

    pub fn from_wallet(&self, wallet: &Wallet, utxo: &UTXO) -> Result<Vec<Movement>, CustomError> {
        let pubkey_hash = wallet.get_pubkey_hash()?;
        let mut pending_movements = vec![];

        for tx in self.tx_set.values() {
            if let Some(mov) = tx.get_movement(&pubkey_hash, utxo)? {
                pending_movements.push(mov);
            }
        }
        Ok(pending_movements)
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        states::wallets_state::Wallets,
        structs::{block_header::BlockHeader, tx_output::TransactionOutput},
    };

    use super::*;

    #[test]
    fn pendings_txs_creation() {
        let pending_txs = PendingTxs::new();
        assert_eq!(pending_txs.tx_set.len(), 0);
        let pending_txs = PendingTxs::default();
        assert_eq!(pending_txs.tx_set.len(), 0);
    }

    #[test]
    fn append_pending_tx() {
        let mut pending_txs = PendingTxs::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        let tx_hash = tx.hash();
        pending_txs.append_pending_tx(tx);
        assert_eq!(pending_txs.tx_set.len(), 1);
        assert_eq!(pending_txs.tx_set.contains_key(&tx_hash), true);
    }

    #[test]
    fn append_existing_pending_tx() {
        let mut pending_txs = PendingTxs::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        let tx_hash = tx.hash();

        let updated = pending_txs.append_pending_tx(tx.clone());
        assert_eq!(updated, true);
        let updated = pending_txs.append_pending_tx(tx);
        assert_eq!(updated, false);

        assert_eq!(pending_txs.tx_set.len(), 1);
        assert_eq!(pending_txs.tx_set.contains_key(&tx_hash), true);
    }

    #[test]
    fn update_pendings() {
        let mut pending_txs = PendingTxs::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };

        let block = Block {
            header: BlockHeader {
                version: 536887296,
                prev_block_hash: vec![],
                merkle_root: vec![],
                timestamp: 1686626483,
                bits: 421617023,
                nonce: 3878826733,
            },
            transactions: vec![tx.clone()],
        };

        let updated = pending_txs.append_pending_tx(tx);
        assert_eq!(updated, true);
        assert_eq!(pending_txs.tx_set.len(), 1);

        pending_txs.update_pending_tx(&block).unwrap();
        assert_eq!(pending_txs.tx_set.len(), 0);
    }

    #[test]
    fn pendings_from_wallet() {
        let mut wallets = Wallets::new("tests/test_wallets.bin".to_string()).unwrap();
        wallets
            .set_active("mhzZUxRkPzNpCsQHemTakuJa5xhCajxyVm")
            .unwrap();

        let mut pending_txs = PendingTxs::new();
        let tx = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![TransactionOutput {
                value: 100,
                script_pubkey: vec![
                    118, 169, 20, 27, 40, 219, 33, 69, 20, 4, 108, 105, 234, 87, 71, 50, 50, 154,
                    22, 16, 220, 64, 85, 136, 172,
                ],
            }],
            lock_time: 0,
        };

        pending_txs.append_pending_tx(tx);

        let pendings_from_wallet = pending_txs
            .from_wallet(
                &wallets.get_active().unwrap(),
                &UTXO::new("tests/test_utxo.bin".to_string()).unwrap(),
            )
            .unwrap();
        assert_eq!(pendings_from_wallet.len(), 1);
        assert_eq!(pendings_from_wallet[0].value, 100);
    }
}
