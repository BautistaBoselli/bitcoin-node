use bitcoin_hashes::{sha256d, Hash};

use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
};

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
    pub fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        if parser.len() < 80 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        Ok(BlockHeader {
            version: parser.extract_i32()?,
            prev_block_hash: parser.extract_buffer(32)?.to_vec(),
            merkle_root: parser.extract_buffer(32)?.to_vec(),
            timestamp: parser.extract_u32()?,
            bits: parser.extract_u32()?,
            nonce: parser.extract_u32()?,
        })
    }
    pub fn hash(&self) -> Vec<u8> {
        sha256d::Hash::hash(&self.serialize())
            .to_byte_array()
            .to_vec()
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
    pub fn parse_headers(buffer: Vec<u8>) -> Result<Vec<BlockHeader>, CustomError> {
        let mut parser = BufferParser::new(buffer);
        if parser.len() % 80 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let mut headers = vec![];
        while !parser.is_empty() {
            headers.push(BlockHeader::parse(parser.extract_buffer(80)?.to_vec())?);
        }
        Ok(headers)
    }
}

impl Message for Headers {
    fn get_command(&self) -> String {
        String::from("headers")
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.headers.len().to_varint_bytes());
        for header in &self.headers {
            buffer.extend(&header.serialize());
            buffer.extend(0_u8.to_le_bytes());
        }
        buffer
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);

        let header_count = parser.extract_varint()?;
        if parser.len() % 81 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let mut headers = vec![];
        while parser.len() >= 81 {
            headers.push(BlockHeader::parse(parser.extract_buffer(81)?.to_vec())?);
        }

        println!("header count: {}", header_count);
        Ok(Headers { headers })
    }
}

// #[cfg(test)]

// mod tests {

//     use super::*;

//     #[test]
// }
