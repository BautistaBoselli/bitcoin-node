use std::vec;

use bitcoin_hashes::{sha256, Hash};

use super::headers::BlockHeader;

use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
};

#[derive(Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    pub fn create_merkle_root(&self) -> Result<(), CustomError> {
        let mut hashes = vec![];
        for transaction in &self.transactions {
            hashes.push(transaction.hash());
        }

        let mut hashes = vec![];
        for transaction in &self.transactions {
            hashes.push(transaction.hash());
        }

        let merkle_tree = merkle_tree(hashes);

        let merkle_root = match merkle_tree.last() {
            Some(root) => root,
            None => return Err(CustomError::InvalidMerkleRoot),
        };

        if merkle_root != &self.header.merkle_root {
            return Err(CustomError::InvalidMerkleRoot);
        }
        Ok(())
    }
}

fn merge_hashes(mut left: Vec<u8>, mut right: Vec<u8>) -> Vec<u8> {
    left.append(&mut right);
    let hash = sha256::Hash::hash(sha256::Hash::hash(left.as_slice()).as_byte_array())
        .as_byte_array()
        .to_vec();
    hash
}

fn merkle_tree(hashes: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if hashes.len() == 1 {
        return hashes;
    }

    let mut level = vec![];
    for i in (0..hashes.len()).step_by(2) {
        let current_hash = hashes.get(i).expect("ERROR ACA 1");
        if i + 1 >= hashes.len() {
            level.push(merge_hashes(current_hash.clone(), current_hash.clone()));
            break;
        }
        let next_hash = hashes.get(i + 1).expect("ERROR ACA 2");
        level.push(merge_hashes(current_hash.clone(), next_hash.clone()));
    }

    merkle_tree(level)
}

impl Message for Block {
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.header.serialize());
        buffer.extend(self.transactions.len().to_varint_bytes());
        for transaction in &self.transactions {
            buffer.extend(transaction.serialize());
        }
        buffer
    }

    fn get_command(&self) -> String {
        String::from("block")
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        let mut parser = BufferParser::new(buffer);
        let header = BlockHeader::parse(parser.extract_buffer(80)?.to_vec(), false)?;
        let tx_count = parser.extract_varint()? as usize;
        let mut transactions = vec![];
        for _ in 0..tx_count {
            let transaction = Transaction::parse(&mut parser)?;
            transactions.push(transaction);
        }

        Ok(Self {
            header,
            transactions,
        })
    }
}

#[derive(Debug)]
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
    pub fn serialize(&self) -> Vec<u8> {
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
}

#[derive(Debug)]
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

#[derive(Debug, Clone)]
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
}

#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn merge_hashes_test() {
        let left_hash: Vec<u8> = vec![1, 2, 3];
        let right_hash: Vec<u8> = vec![4, 5, 6];
        let result = merge_hashes(left_hash, right_hash);
        let hash = sha256::Hash::hash(
            sha256::Hash::hash(vec![1, 2, 3, 4, 5, 6].as_slice()).as_byte_array(),
        )
        .as_byte_array()
        .to_vec();
        assert_eq!(result, hash);
    }

    #[test]
    fn merkle_tree_test() {
        let mut hashes = vec![];
        for i in 0..10 {
            hashes.push(vec![i]);
        }
        println!("{:?}", hashes);
        let result = merkle_tree(hashes);
        assert!(result.len() == 4);
    }

    #[test]
    fn tx_parse() {
        let buffer = vec![
            0x01, 0x00, 0x00, 0x00, 0x01, 0x6D, 0xBD, 0xDB, 0x08, 0x5B, 0x1D, 0x8A, 0xF7, 0x51,
            0x84, 0xF0, 0xBC, 0x01, 0xFA, 0xD5, 0x8D, 0x12, 0x66, 0xE9, 0xB6, 0x3B, 0x50, 0x88,
            0x19, 0x90, 0xE4, 0xB4, 0x0D, 0x6A, 0xEE, 0x36, 0x29, 0x00, 0x00, 0x00, 0x00, 0x8B,
            0x48, 0x30, 0x45, 0x02, 0x21, 0x00, 0xF3, 0x58, 0x1E, 0x19, 0x72, 0xAE, 0x8A, 0xC7,
            0xC7, 0x36, 0x7A, 0x7A, 0x25, 0x3B, 0xC1, 0x13, 0x52, 0x23, 0xAD, 0xB9, 0xA4, 0x68,
            0xBB, 0x3A, 0x59, 0x23, 0x3F, 0x45, 0xBC, 0x57, 0x83, 0x80, 0x02, 0x20, 0x59, 0xAF,
            0x01, 0xCA, 0x17, 0xD0, 0x0E, 0x41, 0x83, 0x7A, 0x1D, 0x58, 0xE9, 0x7A, 0xA3, 0x1B,
            0xAE, 0x58, 0x4E, 0xDE, 0xC2, 0x8D, 0x35, 0xBD, 0x96, 0x92, 0x36, 0x90, 0x91, 0x3B,
            0xAE, 0x9A, 0x01, 0x41, 0x04, 0x9C, 0x02, 0xBF, 0xC9, 0x7E, 0xF2, 0x36, 0xCE, 0x6D,
            0x8F, 0xE5, 0xD9, 0x40, 0x13, 0xC7, 0x21, 0xE9, 0x15, 0x98, 0x2A, 0xCD, 0x2B, 0x12,
            0xB6, 0x5D, 0x9B, 0x7D, 0x59, 0xE2, 0x0A, 0x84, 0x20, 0x05, 0xF8, 0xFC, 0x4E, 0x02,
            0x53, 0x2E, 0x87, 0x3D, 0x37, 0xB9, 0x6F, 0x09, 0xD6, 0xD4, 0x51, 0x1A, 0xDA, 0x8F,
            0x14, 0x04, 0x2F, 0x46, 0x61, 0x4A, 0x4C, 0x70, 0xC0, 0xF1, 0x4B, 0xEF, 0xF5, 0xFF,
            0xFF, 0xFF, 0xFF, 0x02, 0x40, 0x4B, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x19, 0x76,
            0xA9, 0x14, 0x1A, 0xA0, 0xCD, 0x1C, 0xBE, 0xA6, 0xE7, 0x45, 0x8A, 0x7A, 0xBA, 0xD5,
            0x12, 0xA9, 0xD9, 0xEA, 0x1A, 0xFB, 0x22, 0x5E, 0x88, 0xAC, 0x80, 0xFA, 0xE9, 0xC7,
            0x00, 0x00, 0x00, 0x00, 0x19, 0x76, 0xA9, 0x14, 0x0E, 0xAB, 0x5B, 0xEA, 0x43, 0x6A,
            0x04, 0x84, 0xCF, 0xAB, 0x12, 0x48, 0x5E, 0xFD, 0xA0, 0xB7, 0x8B, 0x4E, 0xCC, 0x52,
            0x88, 0xAC, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut parser = BufferParser::new(buffer);
        let tx = Transaction::parse(&mut parser).unwrap();
        assert_eq!(tx.version, 1);
        assert_eq!(tx.inputs.len(), 1);
        let outpoint = tx.inputs.get(0).unwrap().previous_output.clone();
        assert_eq!(
            outpoint.hash,
            vec![
                0x6D, 0xBD, 0xDB, 0x08, 0x5B, 0x1D, 0x8A, 0xF7, 0x51, 0x84, 0xF0, 0xBC, 0x01, 0xFA,
                0xD5, 0x8D, 0x12, 0x66, 0xE9, 0xB6, 0x3B, 0x50, 0x88, 0x19, 0x90, 0xE4, 0xB4, 0x0D,
                0x6A, 0xEE, 0x36, 0x29,
            ]
        );
        let script_sig = tx.inputs.get(0).unwrap().script_sig.clone();
        assert_eq!(script_sig.len(), 139);
        let sequence = tx.inputs.get(0).unwrap().sequence;
        assert_eq!(sequence, 0xFFFFFFFF);
        assert_eq!(tx.outputs.len(), 2);
        let output = tx.outputs.get(0).unwrap();
        assert_eq!(output.value, 5000000);
        let script_pubkey = output.script_pubkey.clone();
        assert_eq!(script_pubkey.len(), 25);
        let output1 = tx.outputs.get(1).unwrap();
        assert_eq!(output1.value, 3354000000);
        let script_pubkey1 = output1.script_pubkey.clone();
        assert_eq!(script_pubkey1.len(), 25);
        assert_eq!(tx.lock_time, 0);
    }
}
