use crate::{error::CustomError, message::Message, parser::BufferParser};

pub struct Ping {
    pub nonce: u64,
}

impl Message for Ping {
    fn get_command(&self) -> String {
        String::from("ping")
    }
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.nonce.to_le_bytes());
        buffer
    }
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer.clone());
        let nonce = parser.extract_u64()?;
        Ok(Ping { nonce })
    }
}

pub struct Pong {
    pub nonce: u64,
}

impl Message for Pong {
    fn get_command(&self) -> String {
        String::from("pong")
    }
    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.nonce.to_le_bytes());
        buffer
    }
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);
        let nonce = parser.extract_u64()?;
        Ok(Pong { nonce })
    }
}
