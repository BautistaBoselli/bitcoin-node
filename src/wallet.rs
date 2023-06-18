use std::collections::HashMap;

use crate::{
    error::CustomError,
    messages::transaction::{OutPoint, TransactionOutput},
    parser::BufferParser,
};

#[derive(Clone, Debug)]
pub struct Movement {
    pub tx_hash: Vec<u8>,
    pub value: u64,
    pub block_hash: Option<Vec<u8>>,
}

impl Movement {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(self.tx_hash.len() as u8);
        buffer.extend(self.tx_hash.clone());
        buffer.extend(self.value.to_le_bytes());
        match self.block_hash.clone() {
            Some(block_hash) => {
                buffer.push(1);
                buffer.push(block_hash.len() as u8);
                buffer.extend(block_hash);
            }
            None => {
                buffer.push(0);
            }
        }
        buffer
    }

    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let tx_hash_len = parser.extract_u8()? as usize;
        let tx_hash = parser.extract_buffer(tx_hash_len)?.to_vec();
        let value = parser.extract_u64()?;
        let block_hash_present = parser.extract_u8()?;
        let block_hash = match block_hash_present {
            0 => None,
            1 => {
                let block_hash_len = parser.extract_u8()? as usize;
                Some(parser.extract_buffer(block_hash_len)?.to_vec())
            }
            _ => {
                return Err(CustomError::Validation(String::from(
                    "Block hash presence incorrectly formatted",
                )))
            }
        };
        Ok(Self {
            tx_hash,
            value,
            block_hash,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Wallet {
    pub name: String,
    pub pubkey: String,
    pub privkey: String,
    pub history: Vec<Movement>,
}

impl Wallet {
    pub fn new(
        name: String,
        pubkey: String,
        privkey: String,
        utxo_set: &HashMap<OutPoint, TransactionOutput>,
    ) -> Result<Self, CustomError> {
        let mut wallet = Self {
            name,
            pubkey,
            privkey,
            history: vec![],
        };
        for (outpoint, output) in utxo_set {
            if output.is_sent_to_key(&wallet.get_pubkey_hash()?) {
                wallet.history.push(Movement {
                    tx_hash: outpoint.hash.clone(),
                    value: output.value,
                    block_hash: None,
                });
            }
        }
        Ok(wallet)
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(self.name.len() as u8);
        buffer.extend(self.name.as_bytes());
        buffer.push(self.pubkey.len() as u8);
        buffer.extend(self.pubkey.as_bytes());
        buffer.push(self.privkey.len() as u8);
        buffer.extend(self.privkey.as_bytes());
        buffer.extend((self.history.len() as u32).to_le_bytes());
        for movement in self.history.clone() {
            buffer.extend(movement.serialize());
        }
        buffer
    }

    pub fn parse_wallets(buffer: Vec<u8>) -> Result<Vec<Self>, CustomError> {
        let mut parser = BufferParser::new(buffer);
        let mut wallets = Vec::new();
        while !parser.is_empty() {
            let name_len = parser.extract_u8()? as usize;
            let name = parser.extract_string(name_len)?;

            let pubkey_len = parser.extract_u8()? as usize;
            let pubkey = parser.extract_string(pubkey_len)?;

            let privkey_len = parser.extract_u8()? as usize;
            let privkey = parser.extract_string(privkey_len)?;

            let history_len = parser.extract_u32()? as usize;
            let mut history = Vec::new();
            for _ in 0..history_len {
                history.push(Movement::parse(&mut parser)?);
            }
            wallets.push(Self {
                name,
                pubkey,
                privkey,
                history,
            });
        }
        Ok(wallets)
    }

    pub fn get_pubkey_hash(&self) -> Result<Vec<u8>, CustomError> {
        let decoded_pubkey = bs58::decode(self.pubkey.clone()).into_vec().map_err(|_| {
            CustomError::Validation(String::from("User PubKey incorrectly formatted"))
        })?;

        match decoded_pubkey.get(1..21) {
            Some(pubkey_hash) => Ok(pubkey_hash.to_vec()),
            None => Err(CustomError::Validation(String::from(
                "User PubKey incorrectly formatted",
            ))),
        }
    }

    pub fn update_history(&mut self, movement: Movement) {
        self.history.push(movement);
    }

    pub fn get_history(&self) -> Vec<Movement> {
        self.history.clone()
    }
}
