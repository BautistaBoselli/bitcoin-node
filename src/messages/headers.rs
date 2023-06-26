use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
    structs::block_header::BlockHeader,
};

#[derive(Debug)]

///Esta estructura es la que se encarga de almacenar los headers de los bloques, esto lo hace en un vector de 'BlockHeaders'
pub struct Headers {
    pub headers: Vec<BlockHeader>,
}

impl Headers {
    /// Esta funcion se encarga de crear un nuevo Headers con un vector vacio de 'BlockHeaders'
    pub fn new() -> Self {
        Headers { headers: vec![] }
    }
}

impl Default for Headers {
    fn default() -> Self {
        Headers::new()
    }
}

/// Implementa el trair Message para Headers
/// Permite serializar, parsear y obtener el comando
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
            headers.push(BlockHeader::parse(
                parser.extract_buffer(81)?.to_vec(),
                true,
            )?);
        }

        if header_count != headers.len() as u64 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        Ok(Headers { headers })
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn parse_and_serialize_headers() {
        let buffer = vec![
            1, 0, 0, 128, 32, 169, 255, 173, 21, 40, 44, 123, 115, 129, 193, 143, 57, 71, 116, 199,
            75, 244, 113, 169, 45, 227, 42, 180, 111, 0, 0, 0, 0, 0, 0, 0, 0, 109, 105, 250, 106,
            92, 126, 17, 171, 97, 243, 124, 194, 172, 252, 249, 166, 202, 8, 231, 136, 21, 107,
            106, 136, 64, 241, 195, 82, 179, 236, 159, 63, 155, 22, 96, 100, 105, 90, 32, 25, 11,
            42, 241, 166, 0,
        ];

        let buffer_clone = buffer.clone();

        let headers = Headers::parse(buffer).unwrap();
        let serialized_headers = headers.serialize();

        assert_eq!(buffer_clone, serialized_headers);
    }

    #[test]
    fn invalid_header() {
        let buffer = vec![
            1, 0, 0, 128, 32, 169, 255, 173, 21, 40, 44, 123, 115, 129, 193, 143, 57, 71, 116, 199,
            75, 244, 113, 169, 45, 227, 42, 180, 111, 0, 0, 0, 0, 0, 0, 0, 0, 109, 105, 250, 106,
            92, 126, 17, 171, 9,
        ];

        let headers = Headers::parse(buffer);

        assert!(headers.is_err());
    }
}
