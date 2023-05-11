use crate::{
    error::CustomError,
    message::Message,
    parser::{BufferParser, VarIntSerialize},
};

// use super::headers::{parse_var_int, serialize_var_int};

#[derive(PartialEq, Debug)]

pub struct GetHeaders {
    pub version: i32,
    pub block_locator_hashes: Vec<Vec<u8>>,
    pub hash_stop: Vec<u8>,
}

impl GetHeaders {
    pub fn new(version: i32, block_locator_hashes: Vec<Vec<u8>>, hash_stop: Vec<u8>) -> Self {
        GetHeaders {
            version,
            block_locator_hashes,
            hash_stop,
        }
    }
}

impl Message for GetHeaders {
    fn get_command(&self) -> String {
        String::from("getheaders")
    }

    fn serialize(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        buffer.extend(&self.version.to_le_bytes());
        buffer.extend(self.block_locator_hashes.len().to_varint_bytes());
        for hash in &self.block_locator_hashes {
            buffer.extend(hash);
        }
        buffer.extend(&self.hash_stop);
        buffer
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError> {
        let mut parser = BufferParser::new(buffer);

        if parser.len() < 37 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let version = parser.extract_i32()?;
        let hash_count = parser.extract_varint()?;

        let mut block_locator_hashes: Vec<Vec<u8>> = vec![];

        println!("hash count: {}", hash_count);
        while parser.len() > 32 {
            let hash = parser.extract_buffer(32)?.to_vec();
            block_locator_hashes.push(hash);
        }

        let hash_stop = parser.extract_buffer(32)?.to_vec();

        if !parser.is_empty() || block_locator_hashes.len() != hash_count as usize {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        println!("hash count: {}", hash_count);
        Ok(GetHeaders {
            version,
            block_locator_hashes,
            hash_stop,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::peer::GENESIS;

    use super::*;

    #[test]
    fn get_headers_serialize() {
        let mut empty_stop_hash: Vec<u8> = vec![];
        empty_stop_hash.resize(32, 0);
        let get_headers = GetHeaders::new(70015, [GENESIS.to_vec()].to_vec(), empty_stop_hash);
        let serialized_getheaders = get_headers.serialize();
        let parsed_getheaders = GetHeaders::parse(serialized_getheaders).unwrap();
        assert_eq!(get_headers, parsed_getheaders);
    }

    #[test]
    fn get_headers_parses_correctly() {
        let serialized_getheaders = vec![
            127, 17, 1, 0, 1, 155, 77, 153, 101, 77, 212, 76, 182, 137, 41, 103, 109, 128, 43, 106,
            32, 200, 118, 162, 103, 247, 127, 103, 118, 167, 48, 41, 155, 158, 132, 88, 193, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0,
        ];

        let parsed_getheaders = GetHeaders::parse(serialized_getheaders).unwrap();
        assert_eq!(parsed_getheaders.version, 70015);
        assert_eq!(parsed_getheaders.block_locator_hashes.len(), 1);
        assert_eq!(
            parsed_getheaders.block_locator_hashes[0],
            [
                155, 77, 153, 101, 77, 212, 76, 182, 137, 41, 103, 109, 128, 43, 106, 32, 200, 118,
                162, 103, 247, 127, 103, 118, 167, 48, 41, 155, 158, 132, 88, 193
            ]
        );
        assert_eq!(parsed_getheaders.hash_stop, vec![0; 32]);
    }

    #[test]
    fn no_headers_get_headers_parses_correctly() {
        let serialized_getheaders = vec![
            127, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let parsed_getheaders = GetHeaders::parse(serialized_getheaders).unwrap();
        assert_eq!(parsed_getheaders.version, 70015);
        assert_eq!(parsed_getheaders.block_locator_hashes.len(), 0);
        assert_eq!(parsed_getheaders.hash_stop, vec![0; 32]);
    }

    #[test]
    fn parse_invalid_getheaders_with_short_buffer() {
        let invalid_getheaders = vec![0; 36];
        let parsed_getheaders = GetHeaders::parse(invalid_getheaders);
        assert!(parsed_getheaders.is_err());
    }

    #[test]
    fn parse_invalid_getheaders_with_incorrect_buffer() {
        let invalid_getheaders = vec![
            127, 17, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let parsed_getheaders = GetHeaders::parse(invalid_getheaders);
        assert!(parsed_getheaders.is_err());
    }
}
