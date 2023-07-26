use std::{
    fs::remove_file,
    io::{Read, Write},
    vec,
};

use bitcoin_hashes::{sha256, Hash};

use super::transaction::Transaction;

use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
    structs::block_header::BlockHeader,
    utils::open_new_file,
};

#[derive(Debug)]

/// Esta estructura es la que se encarga de almacenar los bloques, esto lo hace con un BlockHeader y en un vector de 'transactions' por cada uno
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Esta funcion se encarga de crear un nuevo bloque, recibe un BlockHeader y un vector de 'transactions' para formar al mismo
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    /// Esta funcion se encargar de restaurar un bloque, recibe un path al archivo que contiene al bloque, lo lee y lo parsea
    /// Devuelve CustomError si no puede abrir o leer el archivo
    pub fn restore(path: String) -> Result<Self, CustomError> {
        let mut block_file = open_new_file(path.clone(), true)?;
        let mut block_buffer = Vec::new();
        block_file.read_to_end(&mut block_buffer)?;
        let block = match Self::parse(block_buffer) {
            Ok(block) => Ok(block),
            Err(e) => {
                remove_file(path)?;
                Err(e)
            }
        };
        drop(block_file);
        block
    }

    /// Esta funcion se encarga de guardar un bloque, recibe un path al archivo donde se va a guardar el bloque serializado en bytes
    pub fn save(&self, path: String) -> Result<(), CustomError> {
        let mut block_file = open_new_file(path, true)?;
        block_file.write_all(&self.serialize())?;
        Ok(())
    }

    /// Esta funcion se encarga de crear el merkle tree del bloque, recorre las transacciones del bloque y calcula el hash de cada una, luego que el merkle tree es generado a partir de los hashes de las transacciones, se lo devuelve.
    fn create_merkle_tree(&self) -> Vec<Vec<Vec<u8>>> {
        let mut hashes = vec![];
        for transaction in &self.transactions {
            hashes.push(transaction.hash());
        }

        let mut merkle_tree = vec![hashes.clone()];
        generate_merkle_tree(hashes, &mut merkle_tree);
        merkle_tree
    }

    /// Esta funcion se encarga de validar la proof of inclusion del bloque, creando el merkle tree y comparando el merkle root del BlockHeader con el merkle root calculado
    /// Devuelve CustomError si el merkle root del BlockHeader no coincide con el merkle root calculado, significando que el bloque no es valido
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

    /// Esta funcion se encarga de encontrar el indice de una transaccion dado un bloque y el hash de la transaccion
    /// Devuelve CustomError si no puede encontrar la transaccion en el bloque
    fn find_transaction_index(&self, transaction_hash: &Vec<u8>) -> Result<usize, CustomError> {
        for i in 0..self.transactions.len() {
            if self.transactions[i].hash() == *transaction_hash {
                return Ok(i);
            }
        }
        Err(CustomError::InvalidMerkleRoot)
    }

    /// Esta funcion se encarga de generar el merkle path de una transaccion, recibe el hash de la transaccion y el bloque a la que esta pertence para generar el merkle path
    /// Retorna un vector de bytes con los flags y un vector de vectores de bytes con los hashes de las transacciones necesarias para reconstruir el merkle path
    /// Devuelve CustomError si no puede encontrar la transaccion en el bloque
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

/// Esta funcion se encarga de mergear dos hashes, recibe dos hashes y los mergea en un solo hash
fn merge_hashes(mut left: Vec<u8>, mut right: Vec<u8>) -> Vec<u8> {
    left.append(&mut right);
    let hash = sha256::Hash::hash(sha256::Hash::hash(left.as_slice()).as_byte_array())
        .as_byte_array()
        .to_vec();
    hash
}

/// Esta funcion se encarga de generar el merkle tree, recibe un vector de hashes y un vector de vectores de vectores de bytes, y va generando el merkle tree recursivamente por niveles
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

/// Implementa el trait Message para bloque
/// Permite serializar, parsear y obtener el comando
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
        let header = BlockHeader::parse(parser.extract_buffer(80)?.to_vec())?;
        let tx_count = parser.extract_varint()? as usize;
        let mut transactions = vec![];
        for _ in 0..tx_count {
            let transaction = Transaction::parse_from_parser(&mut parser)?;
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

    use crate::utils::open_new_file;

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

    #[test]
    fn get_command_block_test() {
        let buffer = vec![
            1, 0, 0, 0, 5, 159, 141, 74, 195, 4, 19, 253, 127, 1, 148, 149, 222, 143, 237, 24, 27,
            124, 186, 34, 123, 241, 216, 166, 203, 239, 86, 108, 0, 0, 0, 0, 233, 233, 109, 115,
            249, 241, 6, 200, 176, 73, 10, 24, 28, 209, 102, 159, 255, 179, 239, 72, 185, 225, 10,
            14, 219, 74, 174, 208, 207, 59, 18, 12, 170, 7, 195, 79, 255, 255, 0, 29, 14, 171, 58,
            61,
        ];
        let block_header = BlockHeader::parse(buffer).unwrap();
        let block = Block::new(block_header, vec![]);
        assert_eq!(block.get_command(), "block");
    }
}
