use crate::{
    error::CustomError,
    parser::{BufferParser, VarIntSerialize},
};

use super::outpoint::OutPoint;

#[derive(Debug, Clone, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::BufferParser;

    #[test]
    fn serialize_and_parse() {
        let input = TransactionInput {
            previous_output: OutPoint {
                hash: vec![
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 6,
                    7, 8, 9, 10, 1, 2,
                ],
                index: 0,
            },
            script_sig: vec![4, 5, 6],
            sequence: 0xffffffff,
        };
        let serialize = input.serialize();
        let mut parser = BufferParser::new(serialize);
        let parsed_input = TransactionInput::parse(&mut parser).unwrap();
        assert_eq!(input, parsed_input);
    }
}
