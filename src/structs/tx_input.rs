use crate::{
    error::CustomError,
    parser::{BufferParser, VarIntSerialize},
};

use super::outpoint::OutPoint;

#[derive(Debug, Clone, PartialEq)]

/// Esta estructura representa un input de una transaccion, la cual contiene:
/// - previous_output: Outpoint de la transaccion que genero el input
/// - script_sig: Script que se debe ejecutar para firmar transacciones
/// - sequence: Numero de version definido por el usuario
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

impl TransactionInput {
    /// Esta funcion se encarga de serializar un input en un vector de bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(self.previous_output.serialize());
        buffer.extend(self.script_sig.len().to_varint_bytes());
        buffer.extend(self.script_sig.clone());
        buffer.extend(self.sequence.to_le_bytes());
        buffer
    }

    /// Esta funcion se encarga de parsear un input a partir de un BufferParser.
    /// Devuelve CustomError si:
    /// - Falla alguna de las extracciones del BufferParser
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
        println!("{:?}", parsed_input.clone());
        assert_eq!(input, parsed_input);
    }
}
