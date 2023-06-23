use std::{
    io::{Read, Write},
    vec,
};

use bitcoin_hashes::{sha256, Hash};

use super::{
    headers::{hash_as_string, BlockHeader},
    transaction::Transaction,
};

use crate::{
    error::CustomError,
    message::Message,
    node_state::open_new_file,
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

    pub fn restore(header_hash: String) -> Result<Self, CustomError> {
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", header_hash), true)?;
        let mut block_buffer = Vec::new();
        block_file.read_to_end(&mut block_buffer)?;
        Self::parse(block_buffer)
    }

    pub fn save(&self) -> Result<(), CustomError> {
        let filename = hash_as_string(self.header.hash());
        let mut block_file = open_new_file(format!("store/blocks/{}.bin", filename), true)?;
        block_file.write_all(&self.serialize())?;
        Ok(())
    }

    fn create_merkle_tree(&self) -> Vec<Vec<Vec<u8>>> {
        let mut hashes = vec![];
        for transaction in &self.transactions {
            hashes.push(transaction.hash());
        }

        let mut merkle_tree = vec![hashes.clone()];
        generate_merkle_tree(hashes, &mut merkle_tree);
        merkle_tree
    }

    pub fn create_merkle_root(&self) -> Result<(), CustomError> {
        let merkle_tree = self.create_merkle_tree();

        let merkle_root = match merkle_tree.last() {
            Some(root_level) => root_level[0].to_vec(),
            None => return Err(CustomError::InvalidMerkleRoot),
        };

        if merkle_root != self.header.merkle_root {
            return Err(CustomError::InvalidMerkleRoot);
        }
        Ok(())
    }

    fn find_transaction_index(&self, transaction_hash: &Vec<u8>) -> Result<usize, CustomError> {
        for i in 0..self.transactions.len() {
            if self.transactions[i].hash() == *transaction_hash {
                return Ok(i);
            }
        }
        Err(CustomError::InvalidMerkleRoot)
    }

    pub fn generate_merkle_path(
        &self,
        transaction_hash: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<Vec<u8>>), CustomError> {
        let merkle_tree = self.create_merkle_tree();
        let mut hash_index = self.find_transaction_index(&transaction_hash)?;
        let mut mp_flags: Vec<u8> = vec![1];
        let mut mp_hashes = vec![transaction_hash];

        for level in merkle_tree {
            if level.len() == 1 {
                break;
            }
            if hash_index % 2 == 0 {
                mp_flags.insert(0, 1);
                mp_flags.push(0);
                mp_hashes.push(level[hash_index + 1].clone());
            } else {
                mp_flags.insert(0, 0);
                mp_flags.insert(0, 1);
                mp_hashes.insert(0, level[hash_index - 1].clone());
            }
            hash_index /= 2;
        }

        if mp_flags.len() % 8 != 0 {
            mp_flags.append(&mut vec![0; 8 - mp_flags.len() % 8]);
        }

        Ok((mp_flags, mp_hashes))
    }
}

fn merge_hashes(mut left: Vec<u8>, mut right: Vec<u8>) -> Vec<u8> {
    left.append(&mut right);
    let hash = sha256::Hash::hash(sha256::Hash::hash(left.as_slice()).as_byte_array())
        .as_byte_array()
        .to_vec();
    hash
}

fn generate_merkle_tree(hashes: Vec<Vec<u8>>, merkle_tree: &mut Vec<Vec<Vec<u8>>>) {
    if hashes.len() == 1 {
        return;
    }

    let mut level = vec![];
    for i in (0..hashes.len()).step_by(2) {
        let current_hash = &hashes[i];
        if i + 1 >= hashes.len() {
            level.push(merge_hashes(current_hash.clone(), current_hash.clone()));
            break;
        }
        let next_hash = &hashes[i + 1];
        level.push(merge_hashes(current_hash.clone(), next_hash.clone()));
    }

    merkle_tree.push(level.clone());
    generate_merkle_tree(level, merkle_tree);
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

#[cfg(test)]

mod tests {
    use std::io::Read;

    use crate::node_state::open_new_file;

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
    fn test_merkle_tree() {
        let mut file = open_new_file("tests/test_block.bin".to_string(), true).unwrap();
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).unwrap();
        let block = Block::parse(buffer).unwrap();

        let merkle_tree = block.create_merkle_tree();
        assert_eq!(merkle_tree.last().unwrap()[0], block.header.merkle_root);
        assert!(
            merkle_tree.len() == 6
                && merkle_tree[0].len() == 20
                && merkle_tree[1].len() == 10
                && merkle_tree[2].len() == 5
                && merkle_tree[3].len() == 3
                && merkle_tree[4].len() == 2
                && merkle_tree[5].len() == 1
        );
    }

    #[test]
    fn test_merkle_path() {
        let mut file = open_new_file("tests/test_block.bin".to_string(), true).unwrap();
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).unwrap();
        let block = Block::parse(buffer).unwrap();

        let merkle_tree = block.create_merkle_tree();
        let transactions_hashes = merkle_tree.get(0).unwrap();
        let (flags, hashes) = block
            .generate_merkle_path(transactions_hashes[6].clone())
            .unwrap();

        assert_eq!(flags, vec![1, 1, 1, 0, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(hashes.len(), 6);

        let merging = merge_hashes(hashes[2].clone(), hashes[3].clone());
        let merging = merge_hashes(hashes[1].clone(), merging);
        let merging = merge_hashes(hashes[0].clone(), merging);
        let merging = merge_hashes(merging, hashes[4].clone());
        let merging = merge_hashes(merging, hashes[5].clone());

        assert_eq!(hashes.len(), 6);
        assert_eq!(merging, block.header.merkle_root);
    }       
        
}
