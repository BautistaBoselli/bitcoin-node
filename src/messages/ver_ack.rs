use crate::{error::CustomError, message::Message};

#[derive(Debug)]
pub struct VerAck {}

impl VerAck {
    pub fn new() -> Self {
        VerAck {}
    }
}

impl Default for VerAck {
    fn default() -> Self {
        VerAck::new()
    }
}

impl Message for VerAck {
    fn get_command(&self) -> String {
        String::from("verack")
    }

    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError>
    where
        Self: Sized,
    {
        if !buffer.is_empty() {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        Ok(VerAck {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_verack() {
        let verack = VerAck::new();
        let serialized_verack = verack.serialize();
        assert_eq!(serialized_verack, vec![]);
    }

    #[test]
    fn parse_verack() {
        let verack = VerAck::new();
        let serialized_verack = verack.serialize();
        let parsed_verack = VerAck::parse(serialized_verack);
        assert_eq!(parsed_verack.is_ok(), true);
    }

    #[test]
    fn parse_invalid_verack() {
        let buffer_too_long = vec![0x00];
        let parsed_verack = VerAck::parse(buffer_too_long);
        assert_eq!(parsed_verack.is_err(), true);
    }
}
