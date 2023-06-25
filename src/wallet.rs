use crate::{
    error::CustomError, parser::BufferParser, states::utxo_state::UTXO, structs::movement::Movement,
};

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
        utxo_set: &UTXO,
    ) -> Result<Self, CustomError> {
        if name.is_empty() || pubkey.is_empty() || privkey.is_empty() {
            return Err(CustomError::Validation(
                "Name, public key and private key must not be empty".to_string(),
            ));
        }
        if pubkey.len() != 34 {
            return Err(CustomError::Validation(
                "Public key must be 34 characters long".to_string(),
            ));
        }
        let mut wallet = Self {
            name,
            pubkey,
            privkey,
            history: vec![],
        };
        for (outpoint, value) in &utxo_set.tx_set {
            if value.tx_out.is_sent_to_key(&wallet.get_pubkey_hash()?)? {
                wallet.history.push(Movement {
                    tx_hash: outpoint.hash.clone(),
                    value: value.tx_out.value,
                    block_hash: Some(value.block_hash.clone()),
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

    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let name_len = parser.extract_u8()? as usize;
        let name = parser.extract_string(name_len)?;

        let pubkey_len = parser.extract_u8()? as usize;
        let pubkey = parser.extract_string(pubkey_len)?;

        let privkey_len = parser.extract_u8()? as usize;
        let privkey = parser.extract_string(privkey_len)?;

        let history_len = parser.extract_u32()? as usize;
        let mut history = Vec::new();
        for _ in 0..history_len {
            history.push(Movement::parse(parser)?);
        }

        Ok(Self {
            name,
            pubkey,
            privkey,
            history,
        })
    }

    pub fn get_pubkey_hash(&self) -> Result<Vec<u8>, CustomError> {
        get_pubkey_hash(self.pubkey.clone())
    }

    pub fn get_privkey_hash(&self) -> Result<Vec<u8>, CustomError> {
        get_privkey_hash(self.privkey.clone())
    }

    pub fn get_script_pubkey(&self) -> Result<Vec<u8>, CustomError> {
        get_script_pubkey(self.pubkey.clone())
    }

    pub fn update_history(&mut self, movement: Movement) {
        self.history.push(movement);
    }

    pub fn get_history(&self) -> Vec<Movement> {
        self.history.clone()
    }
}

pub fn get_pubkey_hash(pubkey: String) -> Result<Vec<u8>, CustomError> {
    let decoded_pubkey = bs58::decode(pubkey)
        .into_vec()
        .map_err(|_| CustomError::Validation(String::from("User PubKey incorrectly formatted")))?;

    match decoded_pubkey.get(1..21) {
        Some(pubkey_hash) => Ok(pubkey_hash.to_vec()),
        None => Err(CustomError::Validation(String::from(
            "User PubKey incorrectly formatted",
        ))),
    }
}

pub fn get_privkey_hash(privkey: String) -> Result<Vec<u8>, CustomError> {
    let decoded_privkey = bs58::decode(privkey)
        .into_vec()
        .map_err(|_| CustomError::Validation(String::from("User PrivKey incorrectly formatted")))?;

    match decoded_privkey.get(1..33) {
        Some(pubkey_hash) => Ok(pubkey_hash.to_vec()),
        None => Err(CustomError::Validation(String::from(
            "User PubKey incorrectly formatted",
        ))),
    }
}

pub fn get_script_pubkey(pubkey: String) -> Result<Vec<u8>, CustomError> {
    let mut script_pubkey = Vec::new();
    script_pubkey.push(0x76);
    script_pubkey.push(0xa9);
    script_pubkey.push(0x14);
    script_pubkey.extend(get_pubkey_hash(pubkey)?);
    script_pubkey.push(0x88);
    script_pubkey.push(0xac);
    Ok(script_pubkey)
}

#[cfg(test)]

mod tests {
    use crate::structs::movement::Movement;

    use super::*;

    #[test]
    fn wallet_creation() {
        let utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test"),
            String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"),
            String::from("privkey"),
            &utxo_set,
        )
        .unwrap();
        assert_eq!(wallet.name, String::from("test"));
        assert_eq!(
            wallet.pubkey,
            String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu")
        );
        assert_eq!(wallet.privkey, String::from("privkey"));
        assert_eq!(wallet.history.len(), 0);
    }

    #[test]
    fn wallet_creation_with_invalid_pubkey() {
        let utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test"),
            String::from("invalid_pubkey"),
            String::from("privkey"),
            &utxo_set,
        );
        assert_eq!(wallet.is_err(), true);
    }

    #[test]
    fn wallet_creation_with_no_name() {
        let utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from(""),
            String::from("pubkey"),
            String::from("privkey"),
            &utxo_set,
        );
        assert_eq!(wallet.is_err(), true);
    }

    #[test]
    fn wallet_creation_with_no_pubkey() {
        let utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test"),
            String::from(""),
            String::from("privkey"),
            &utxo_set,
        );
        assert_eq!(wallet.is_err(), true);
    }

    #[test]
    fn wallet_creation_with_no_privkey() {
        let utxo_set = UTXO::new(String::from("tests/test_utxo.bin")).unwrap();
        let wallet = Wallet::new(
            String::from("test"),
            String::from("pubkey"),
            String::from(""),
            &utxo_set,
        );
        assert_eq!(wallet.is_err(), true);
    }

    #[test]
    fn wallet_serialization() {
        let wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("pubkey"),
            privkey: String::from("privkey"),
            history: vec![],
        };
        let serialized_wallet = wallet.serialize();
        let mut parser = BufferParser::new(serialized_wallet);
        let parsed_wallet = Wallet::parse(&mut parser).unwrap();
        assert_eq!(parsed_wallet.name, String::from("test"));
        assert_eq!(parsed_wallet.pubkey, String::from("pubkey"));
        assert_eq!(parsed_wallet.privkey, String::from("privkey"));
    }

    #[test]
    fn wallet_history_serialization() {
        let mut wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("pubkey"),
            privkey: String::from("privkey"),
            history: vec![],
        };
        wallet.update_history(Movement {
            tx_hash: vec![
                158, 58, 146, 241, 218, 207, 194, 196, 103, 192, 89, 27, 56, 110, 195, 138, 29,
                177, 167, 47, 144, 191, 102, 68, 45, 70, 88, 237, 140, 224, 130, 115,
            ],
            value: 500,
            block_hash: Some(vec![
                167, 131, 118, 190, 70, 199, 31, 2, 255, 135, 123, 36, 232, 182, 60, 178, 98, 181,
                242, 112, 111, 183, 22, 128, 11, 0, 0, 0, 0, 0, 0, 0,
            ]),
        });
        let serialized_wallet = wallet.serialize();
        let mut parser = BufferParser::new(serialized_wallet);
        let parsed_wallet = Wallet::parse(&mut parser).unwrap();
        assert_eq!(
            parsed_wallet.history[0].block_hash,
            Some(vec![
                167, 131, 118, 190, 70, 199, 31, 2, 255, 135, 123, 36, 232, 182, 60, 178, 98, 181,
                242, 112, 111, 183, 22, 128, 11, 0, 0, 0, 0, 0, 0, 0
            ])
        );
    }

    #[test]
    fn wallet_pubkey_hash() {
        let wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"),
            privkey: String::from("privkey"),
            history: vec![],
        };
        let pubkey_hash = wallet.get_pubkey_hash().unwrap();
        assert_eq!(
            pubkey_hash,
            vec![
                132, 178, 35, 78, 47, 170, 110, 26, 117, 29, 126, 82, 132, 235, 16, 204, 230, 247,
                81, 246
            ]
        );
    }

    #[test]
    fn wallet_script_pubkey() {
        let wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("mscatccDgq7azndWHFTzvEuZuywCsUvTRu"),
            privkey: String::from("privkey"),
            history: vec![],
        };
        let script_pubkey = wallet.get_script_pubkey().unwrap();
        assert_eq!(
            script_pubkey,
            vec![
                118, 169, 20, 132, 178, 35, 78, 47, 170, 110, 26, 117, 29, 126, 82, 132, 235, 16,
                204, 230, 247, 81, 246, 136, 172
            ]
        );
    }

    #[test]
    fn wallet_privkey_hash() {
        let wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("pubkey"),
            privkey: String::from("cNpwEsaVLhju18SJowLtdCNaJtvMvqL4jtFLm2FXw7vZjg4sRWvH"),
            history: vec![],
        };
        let privkey_hash = wallet.get_privkey_hash().unwrap();
        assert_eq!(
            privkey_hash,
            vec![
                37, 40, 250, 211, 140, 107, 40, 1, 172, 178, 73, 96, 107, 232, 139, 51, 193, 141,
                214, 94, 111, 179, 212, 131, 164, 214, 178, 10, 225, 183, 223, 54
            ]
        );
    }

    #[test]
    fn wallet_incorrect_privkey_hash() {
        let wallet = Wallet {
            name: String::from("test"),
            pubkey: String::from("pubkey"),
            privkey: String::from("test"),
            history: vec![],
        };
        let privkey_hash = wallet.get_privkey_hash();
        assert!(privkey_hash.is_err());
    }
}
