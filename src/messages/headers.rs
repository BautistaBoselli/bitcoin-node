use crate::{error::CustomError, message::Message};

#[derive(Debug)]

pub struct Headers {
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
}

impl BlockHeader {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.version.to_le_bytes());
        buffer.extend(&self.prev_block_hash);
        buffer.extend(&self.merkle_root);
        buffer.extend(&self.timestamp.to_le_bytes());
        buffer.extend(&self.bits.to_le_bytes());
        buffer.extend(&self.nonce.to_le_bytes());
        buffer
    }
    pub fn parse(buffer: Vec<u8>) -> Self {
        let version = i32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let prev_block_hash = buffer[4..36].to_vec();
        let merkle_root = buffer[36..68].to_vec();
        let timestamp = u32::from_le_bytes([buffer[68], buffer[69], buffer[70], buffer[71]]);
        let bits = u32::from_le_bytes([buffer[72], buffer[73], buffer[74], buffer[75]]);
        let nonce = u32::from_le_bytes([buffer[76], buffer[77], buffer[78], buffer[79]]);
        BlockHeader {
            version,
            prev_block_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        }
    }
}

impl Headers {
    pub fn new() -> Self {
        Headers { headers: vec![] }
    }
    pub fn serialize_headers(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        for header in &self.headers {
            let header_buffer: Vec<u8> = header.serialize();
            buffer.extend(header_buffer);
        }
        buffer
    }
    pub fn parse_headers(buffer: Vec<u8>) -> Vec<BlockHeader> {
        let mut headers = vec![];
        let mut i = 0;
        while i < buffer.len() {
            headers.push(BlockHeader::parse(buffer[i..(i + 80)].to_vec()));
            i += 80;
        }
        headers
    }
}

impl Message for Headers {
    fn get_command(&self) -> String {
        String::from("headers")
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(serialize_var_int(self.headers.len() as u64));
        for header in &self.headers {
            buffer.extend(&header.serialize());
            buffer.extend((0 as u8).to_le_bytes());
        }
        buffer
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        if buffer.len() < 9 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let (header_count, mut i) = parse_var_int(&buffer);

        if (buffer.len() - i) % 81 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        println!("header count: {}", header_count);

        let mut headers = vec![];
        while i < buffer.len() {
            headers.push(BlockHeader::parse(buffer[i..(i + 81)].to_vec()));
            i += 81;
        }
        Ok(Headers { headers })
    }
}

pub fn parse_var_int(buffer: &Vec<u8>) -> (u64, usize) {
    let (header_count, i) = match buffer[0] {
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
    (header_count, i)
}

pub fn serialize_var_int(var_int: u64) -> Vec<u8> {
    if var_int < 0xFD {
        return (var_int as u8).to_le_bytes().to_vec();
    }
    if var_int <= 0xFFFF {
        let mut buffer = [0xFD as u8].to_vec();
        buffer.append(&mut (var_int as u16).to_le_bytes().to_vec());
        return buffer;
    }
    if var_int <= 0xFFFFFFFF {
        let mut buffer = [0xFE as u8].to_vec();
        buffer.append(&mut (var_int as u32).to_le_bytes().to_vec());
        return buffer;
    }
    let mut buffer = [0xFF as u8].to_vec();
    buffer.append(&mut var_int.to_le_bytes().to_vec());
    buffer
}

// #[cfg(test)]

// mod tests {

//     use super::*;

//     #[test]
// }
