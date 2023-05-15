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

#[derive(Debug)]
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

#[derive(Debug)]
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
