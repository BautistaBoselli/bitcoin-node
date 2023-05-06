use crate::{error::CustomError, message::Message};

#[derive(Debug)]

pub struct Headers {
    pub header_count: u64,
    pub headers: Vec<BlockHeader>,
}

#[derive(Debug)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block_hash: Vec<u8>,
    pub merkle_root: Vec<u8>,
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    pub tx_count: u8,
}

impl Headers {
    pub fn new(header_count: u64) -> Self {
        Headers {
            header_count,
            headers: vec![],
        }
    }
}

impl Message for Headers {
    fn get_command(&self) -> String {
        String::from("headers")
    }

    fn serialize(&self) -> Vec<u8> {
        /*
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.header_count.to_le_bytes());
        for header in &self.headers {
            buffer.extend(&header.version.to_le_bytes());
            buffer.extend(&header.prev_block_hash);
            buffer.extend(&header.merkle_root);
            buffer.extend(&header.timestamp.to_le_bytes());
            buffer.extend(&header.bits.to_le_bytes());
            buffer.extend(&header.nonce.to_le_bytes());
            buffer.extend(&header.tx_count.to_le_bytes());
        }
        buffer
        */
        vec![]
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        println!("longitud: {}", buffer.len() % 81);
        println!("buffer 0: {}", buffer[0]);
        println!("buffer 1: {}", buffer[1]);

        let (header_count, mut i) = match buffer[0] {
            0xFF => (
                u64::from_le_bytes([
                    buffer[1], buffer[2], buffer[3], buffer[4], buffer[5], buffer[6], buffer[7],
                    buffer[8],
                ]),
                9,
            ),
            0xFE => (
                u64::from_le_bytes([buffer[1], buffer[2], buffer[3], 0, 0, 0, 0, 0]),
                5,
            ),
            0xFD => (
                u64::from_le_bytes([buffer[1], buffer[2], 0, 0, 0, 0, 0, 0]),
                3,
            ),
            _ => (u64::from_le_bytes([buffer[0], 0, 0, 0, 0, 0, 0, 0]), 1),
        };

        println!("header count: {}", header_count);
        let mut headers: Vec<BlockHeader> = vec![];
        while i < buffer.len() {
            let version =
                i32::from_le_bytes([buffer[i], buffer[i + 1], buffer[i + 2], buffer[i + 3]]);
            let prev_block_hash = buffer[(i + 4)..(i + 36)].to_vec();
            let merkle_root = buffer[(i + 36)..(i + 68)].to_vec();
            let timestamp = u32::from_le_bytes([
                buffer[i + 68],
                buffer[i + 69],
                buffer[i + 70],
                buffer[i + 71],
            ]);
            let bits = u32::from_le_bytes([
                buffer[i + 72],
                buffer[i + 73],
                buffer[i + 74],
                buffer[i + 75],
            ]);
            let nonce = u32::from_le_bytes([
                buffer[i + 76],
                buffer[i + 77],
                buffer[i + 78],
                buffer[i + 79],
            ]);
            let tx_count = u8::from_le_bytes([buffer[i + 80]]);
            headers.push(BlockHeader {
                version,
                prev_block_hash,
                merkle_root,
                timestamp,
                bits,
                nonce,
                tx_count,
            });
            i += 81;
        }
        Ok(Headers {
            header_count,
            headers,
        })
    }
}

// #[cfg(test)]

// mod tests {

//     use super::*;

//     #[test]
// }
