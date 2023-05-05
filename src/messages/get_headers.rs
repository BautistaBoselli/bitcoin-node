use crate::{error::CustomError, message::Message};

#[derive(PartialEq, Debug)]

pub struct GetHeaders {
    pub version: i32,
    pub block_locator_hashes: Vec<Vec<u8>>,
    pub hash_stop: Vec<u8>,
}

impl GetHeaders {
    pub fn new(version: i32, hash_stop: Vec<u8>) -> Self {
        GetHeaders {
            version,
            block_locator_hashes: vec![],
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
        buffer.extend(&self.block_locator_hashes.len().to_le_bytes());
        for hash in &self.block_locator_hashes {
            buffer.extend(hash);
        }
        buffer.extend(&self.hash_stop);
        buffer
    }

    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        if buffer.len() < 37 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let version = i32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let hash_count = u8::from_le_bytes([buffer[4]]);
        let mut block_locator_hashes: Vec<Vec<u8>> = vec![];
        if hash_count == 0 {
            return Ok(GetHeaders {
                version,
                block_locator_hashes,
                hash_stop: buffer[5..37].to_vec(),
            });
        }
        let mut i = 5;
        while i < buffer.len() - 32 {
            let hash = buffer[i..(i + 32)].to_vec();
            block_locator_hashes.push(hash);
            i += 32;
        }
        if i != buffer.len() - 32 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let hash_stop = buffer[i..(i + 32)].to_vec();

        if block_locator_hashes.len() != hash_count as usize {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        Ok(GetHeaders {
            version,
            block_locator_hashes,
            hash_stop,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn get_headers_serialize() {
        let mut empty_stop_hash: Vec<u8> = vec![];
        empty_stop_hash.resize(32, 0);
        let get_headers = GetHeaders::new(70015, empty_stop_hash);
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
