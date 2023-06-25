use crate::{error::CustomError, parser::BufferParser};

#[derive(Clone, Debug)]
pub struct Movement {
    pub tx_hash: Vec<u8>,
    pub value: u64,
    pub block_hash: Option<Vec<u8>>,
}

impl Movement {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.push(self.tx_hash.len() as u8);
        buffer.extend(self.tx_hash.clone());
        buffer.extend(self.value.to_le_bytes());
        match self.block_hash.clone() {
            Some(block_hash) => {
                buffer.push(1);
                buffer.push(block_hash.len() as u8);
                buffer.extend(block_hash);
            }
            None => {
                buffer.push(0);
            }
        }
        buffer
    }

    pub fn parse(parser: &mut BufferParser) -> Result<Self, CustomError> {
        let tx_hash_len = parser.extract_u8()? as usize;
        let tx_hash = parser.extract_buffer(tx_hash_len)?.to_vec();
        let value = parser.extract_u64()?;
        let block_hash_present = parser.extract_u8()?;
        let block_hash = match block_hash_present {
            0 => None,
            1 => {
                let block_hash_len = parser.extract_u8()? as usize;
                Some(parser.extract_buffer(block_hash_len)?.to_vec())
            }
            _ => {
                return Err(CustomError::Validation(String::from(
                    "Block hash presence incorrectly formatted",
                )))
            }
        };

        Ok(Self {
            tx_hash,
            value,
            block_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::BufferParser, structs::movement::Movement};

    #[test]
    fn movement_serialization() {
        let movement = Movement {
            tx_hash: vec![
                158, 58, 146, 241, 218, 207, 194, 196, 103, 192, 89, 27, 56, 110, 195, 138, 29,
                177, 167, 47, 144, 191, 102, 68, 45, 70, 88, 237, 140, 224, 130, 115,
            ],
            value: 500,
            block_hash: Some(vec![
                167, 131, 118, 190, 70, 199, 31, 2, 255, 135, 123, 36, 232, 182, 60, 178, 165, 110,
                47, 11, 50, 1, 133, 106, 59, 195, 153, 210, 59, 21, 163, 41,
            ]),
        };
        let serialized_movement = movement.serialize();
        let mut parser = BufferParser::new(serialized_movement);
        let parsed_movement = Movement::parse(&mut parser).unwrap();
        assert_eq!(
            parsed_movement.tx_hash,
            vec![
                158, 58, 146, 241, 218, 207, 194, 196, 103, 192, 89, 27, 56, 110, 195, 138, 29,
                177, 167, 47, 144, 191, 102, 68, 45, 70, 88, 237, 140, 224, 130, 115
            ]
        );
        assert_eq!(parsed_movement.value, 500);
        assert_eq!(
            parsed_movement.block_hash,
            Some(vec![
                167, 131, 118, 190, 70, 199, 31, 2, 255, 135, 123, 36, 232, 182, 60, 178, 165, 110,
                47, 11, 50, 1, 133, 106, 59, 195, 153, 210, 59, 21, 163, 41
            ])
        );
    }

    #[test]
    fn movement_without_block_hash() {
        let movement = Movement {
            tx_hash: vec![
                158, 58, 146, 241, 218, 207, 194, 196, 103, 192, 89, 27, 56, 110, 195, 138, 29,
                177, 167, 47, 144, 191, 102, 68, 45, 70, 88, 237, 140, 224, 130, 115,
            ],
            value: 500,
            block_hash: None,
        };
        let serialized_movement = movement.serialize();
        let mut parser = BufferParser::new(serialized_movement);
        let parsed_movement = Movement::parse(&mut parser).unwrap();
        assert_eq!(
            parsed_movement.tx_hash,
            vec![
                158, 58, 146, 241, 218, 207, 194, 196, 103, 192, 89, 27, 56, 110, 195, 138, 29,
                177, 167, 47, 144, 191, 102, 68, 45, 70, 88, 237, 140, 224, 130, 115
            ]
        );
        assert_eq!(parsed_movement.value, 500);
        assert_eq!(parsed_movement.block_hash, None);
    }
}
