use std::collections::HashMap;

use bitcoin_hashes::{sha256, sha256d, Hash};
use secp256k1::Secp256k1;

use crate::{
    error::CustomError,
    message::Message,
    messages::headers::hash_as_string,
    parser::{BufferParser, VarIntSerialize},
    utxo::UTXO,
    wallet::{get_script_pubkey, Movement, Wallet},
};

const SIGHASH_ALL: u32 = 1;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub version: u32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u32,
}

impl Transaction {
    pub fn hash(&self) -> Vec<u8> {
        sha256::Hash::hash(sha256::Hash::hash(self.serialize().as_slice()).as_byte_array())
            .as_byte_array()
            .to_vec()
    }
    // pub fn serialize(&self) -> Vec<u8> {
    //     let mut buffer: Vec<u8> = vec![];
    //     buffer.extend(self.version.to_le_bytes());
    //     buffer.extend(self.inputs.len().to_varint_bytes());
    //     for input in &self.inputs {
    //         buffer.extend(input.serialize());
    //     }
    //     buffer.extend(self.outputs.len().to_varint_bytes());
    //     for output in &self.outputs {
    //         buffer.extend(output.serialize());
    //     }
    //     buffer.extend(self.lock_time.to_le_bytes());

    //     buffer
    // }

    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let version = parser.extract_u32()?;
        //chequear lo del flag
        let tx_in_count = parser.extract_varint()? as usize;
        let mut inputs = vec![];
        for _ in 0..tx_in_count {
            inputs.push(TransactionInput::parse(parser)?);
        }
        let tx_out_count = parser.extract_varint()? as usize;
        let mut outputs = vec![];
        for _ in 0..tx_out_count {
            outputs.push(TransactionOutput::parse(parser)?);
        }

        let lock_time = parser.extract_u32()?;
        Ok(Self {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }

    pub fn get_movement(&self, public_key_hash: &Vec<u8>, utxo: &UTXO) -> Option<Movement> {
        let mut value = 0;

        for output in &self.outputs {
            if output.is_sent_to_key(public_key_hash) {
                value += output.value;
            }
        }
        for input in &self.inputs {
            if let Some(utxo_value) = utxo.tx_set.get(&input.previous_output) {
                if utxo_value.tx_out.is_sent_to_key(public_key_hash) {
                    value -= utxo_value.tx_out.value;
                }
            }
        }
        if value != 0 {
            Some(Movement {
                tx_hash: self.hash(),
                value,
                block_hash: None,
            })
        } else {
            None
        }
    }

    pub fn create(
        sender_wallet: &Wallet,
        inputs_outpoints: Vec<OutPoint>,
        outputs: HashMap<String, u64>,
    ) -> Result<Self, CustomError> {
        // println!("Wallet: {:?}", sender_wallet);
        // println!("Inputs: {:?}", inputs_outpoints);
        // println!("Outputs: {:?}", outputs);
        let mut transaction = Transaction {
            version: 1,
            inputs: vec![],
            outputs: vec![],
            lock_time: 0,
        };
        let script_pubkey = sender_wallet.get_script_pubkey()?;
        //println!("script pubkey: {:?}", script_pubkey);
        for outpoint in inputs_outpoints {
            let input = TransactionInput {
                previous_output: outpoint,
                script_sig: script_pubkey.clone(),
                sequence: 0xffffffff,
            };
            transaction.inputs.push(input);
        }
        for (pubkey, value) in outputs {
            let script_pubkey = get_script_pubkey(pubkey)?;
            let output = TransactionOutput {
                value,
                script_pubkey,
            };
            transaction.outputs.push(output);
        }

        transaction.sign(sender_wallet)?;

        // println!(
        //     "Transaction: {:?}",
        //     hash_as_string(transaction.serialize()).to_ascii_lowercase()
        // );

        Ok(transaction)
    }

    fn sign(&mut self, wallet: &Wallet) -> Result<(), CustomError> {
        let privkey_hash = wallet.get_privkey_hash()?;

        //println!("privkey hash ({}): {:?}", privkey_hash.len(), privkey_hash);

        let serialized_unsigned_tx = self.serialize();

        let script_sig = sign(serialized_unsigned_tx, &privkey_hash)?;

        for input in &mut self.inputs {
            input.script_sig = script_sig.clone();
        }

        Ok(())
    }
}

impl Message for Transaction {
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.version.to_le_bytes());
        buffer.extend(self.inputs.len().to_varint_bytes());
        for input in &self.inputs {
            buffer.extend(input.serialize());
        }
        buffer.extend(self.outputs.len().to_varint_bytes());
        for output in &self.outputs {
            buffer.extend(output.serialize());
        }
        buffer.extend(self.lock_time.to_le_bytes());

        buffer
    }

    fn get_command(&self) -> String {
        String::from("tx")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        let mut parser = BufferParser::new(buffer);

        let version = parser.extract_u32()?;
        //chequear lo del flag
        let tx_in_count = parser.extract_varint()? as usize;
        let mut inputs = vec![];
        for _ in 0..tx_in_count {
            inputs.push(TransactionInput::parse(&mut parser)?);
        }
        let tx_out_count = parser.extract_varint()? as usize;
        let mut outputs = vec![];
        for _ in 0..tx_out_count {
            outputs.push(TransactionOutput::parse(&mut parser)?);
        }

        let lock_time = parser.extract_u32()?;
        Ok(Self {
            version,
            inputs,
            outputs,
            lock_time,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TransactionInput {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.previous_output.serialize());
        buffer.extend(self.script_sig.len().to_varint_bytes());
        buffer.extend(self.script_sig.clone());
        buffer.extend(self.sequence.to_le_bytes());
        buffer
    }
    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let previous_output = OutPoint::parse(parser.extract_buffer(36)?.to_vec())?;
        let script_sig_length = parser.extract_varint()? as usize;
        let script_sig = parser.extract_buffer(script_sig_length)?.to_vec();
        let sequence = parser.extract_u32()?;
        Ok(Self {
            previous_output,
            script_sig,
            sequence,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct OutPoint {
    pub hash: Vec<u8>,
    pub index: u32,
}

impl OutPoint {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.hash.clone());
        buffer.extend(self.index.to_le_bytes());
        buffer
    }
    pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        let hash = parser.extract_buffer(32)?.to_vec();
        let index = parser.extract_u32()?;
        Ok(Self { hash, index })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionOutput {
    pub value: u64,
    pub script_pubkey: Vec<u8>,
}

impl TransactionOutput {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.value.to_le_bytes());
        buffer.extend(self.script_pubkey.len().to_varint_bytes());
        buffer.extend(self.script_pubkey.clone());
        buffer
    }

    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let value = parser.extract_u64()?;
        let script_pk_length = parser.extract_varint()? as usize;
        let script_pubkey = parser.extract_buffer(script_pk_length)?.to_vec();
        Ok(Self {
            value,
            script_pubkey,
        })
    }

    pub fn is_sent_to_key(&self, public_key_hash: &Vec<u8>) -> bool {
        let parser = &mut BufferParser::new(self.script_pubkey.clone());
        match parser.extract_u8() {
            Ok(0x76) => compare_p2pkh(parser, public_key_hash),
            _ => false,
        }
    }
}

fn compare_p2pkh(parser: &mut BufferParser, public_key_hash: &Vec<u8>) -> bool {
    match parser.extract_u8() {
        Ok(0xa9) => (),
        _ => return false,
    }
    match parser.extract_u8() {
        Ok(0x14) => (),
        _ => return false,
    }
    let hash = parser.extract_buffer(20).unwrap().to_vec();

    hash == *public_key_hash
}

fn sign(mut buffer: Vec<u8>, privkey: &Vec<u8>) -> Result<Vec<u8>, CustomError> {
    buffer.extend(SIGHASH_ALL.to_le_bytes());

    println!("buffer: {:?}", hash_as_string(buffer.clone()));

    let z = sha256d::Hash::hash(&buffer);

    let secp = Secp256k1::new();
    let msg = secp256k1::Message::from_slice(&z.to_byte_array())
        .map_err(|_| CustomError::CannotSignTx)?;

    let key = secp256k1::SecretKey::from_slice(&privkey).map_err(|_| CustomError::CannotSignTx)?;
    let publickey = secp256k1::PublicKey::from_secret_key(&secp, &key).serialize();

    let signature = secp.sign_ecdsa(&msg, &key).serialize_der();

    let mut script_sig = vec![];

    script_sig.extend((signature.len() + 1).to_varint_bytes());
    script_sig.extend(signature.to_vec());
    script_sig.extend((0x1 as u8).to_le_bytes());
    script_sig.extend(publickey.len().to_varint_bytes());
    script_sig.extend(publickey.clone());

    Ok(script_sig)
}

mod tests {
    // use crate::wallet::Wallet;

    // use super::{Transaction, OutPoint};

    // #[test]
    // fn test_sign() {
    //     let wallet = Wallet {
    //         name: "".to_string(),
    //         pubkey: "mwjQy4mHvLC4iv6VKJWR9LrQgTW3MqPhrH".to_string(),
    //         privkey: "1234".to_string(),
    //         history: vec![],
    //     };
    //     let transaction = Transaction::create(
    //         &wallet,
    //         vec![OutPoint {
    //             hash: "53576a80ea79a6ed8f83abdb89589d98847f6be0c919660cf2d805a6b1058ec0".to_string(),
    //             index: 1,
    //         }],
    //         vec![("tb1ql6g96n50lqgmhrryxryzfau2zp2ucvzt4s29wc".to_string(), 100)],
    //     )
    // }
}
