//use super::headers::{serialize_var_int, BlockHeader};

// pub struct Block {
//     pub header: BlockHeader,
//     pub transactions: Vec<Vec<u8>>,
// }

// impl Block {
//     pub fn new(header: BlockHeader, transactions: Vec<Vec<u8>>) -> Self {
//         Self {
//             header,
//             transactions,
//         }
//     }
// }

// impl Message for Block {
//     fn serialize(&self) -> Vec<u8> {
//         let mut buffer: Vec<u8> = vec![];
//         buffer.extend(self.header.serialize());
//         buffer.extend(serialize_var_int(self.transactions.len() as u64));
//         for transaction in &self.transactions {
//             buffer.extend(transaction.serialize());
//         }
//         buffer
//     }

//     fn get_command(&self) -> String {
//         String::from("block")
//     }

//     fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError>
//     where
//         Self: Sized,
//     {
//         let header = BlockHeader::parse(buffer[0..80].to_vec())?;
//         let (transaction_count, mut i) = parse_var_int(&buffer[80..]);
//         let mut transactions = vec![];
//         while i < buffer.len() {
//             let transaction = Transaction::parse(buffer[i..].to_vec())?;
//             transactions.push(transaction);
//             i += 1;
//         }
//         Ok(Self {
//             header,
//             transactions,
//         })
//     }
// }
