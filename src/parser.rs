use std::net::{Ipv6Addr, SocketAddrV6};

use crate::error::CustomError;

pub struct BufferParser {
    buffer: Vec<u8>,
    pos: usize,
}

impl BufferParser {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, pos: 0 }
    }
    pub fn get_pos(&mut self) -> usize {
        self.pos
    }

    pub fn go_foward(&mut self, pos: usize) {
        if pos + self.pos > self.buffer.len() {
            self.pos = self.buffer.len();
            return;
        }
        self.pos += pos;
    }

    pub fn go_backward(&mut self, pos: usize) {
        if pos > self.pos {
            self.pos = 0;
            return;
        }
        self.pos -= pos;
    }

    pub fn total_len(&mut self) -> usize {
        self.buffer.len()
    }

    pub fn len(&mut self) -> usize {
        self.buffer.len() - self.pos
    }

    pub fn is_empty(&mut self) -> bool {
        self.buffer.len() - self.pos == 0
    }

    pub fn extract_buffer(&mut self, size: usize) -> Result<&[u8], CustomError> {
        let buffer = match self.buffer.get(self.pos..(self.pos + size)) {
            Some(buffer) => Ok(buffer),
            None => return Err(CustomError::SerializedBufferIsInvalid),
        };
        self.pos += size;
        buffer
    }

    pub fn extract_u8(&mut self) -> Result<u8, CustomError> {
        let slice: [u8; 1] = self
            .extract_buffer(1)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(u8::from_le_bytes(slice))
    }
    pub fn extract_u16(&mut self) -> Result<u16, CustomError> {
        let slice: [u8; 2] = self
            .extract_buffer(2)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(u16::from_le_bytes(slice))
    }
    pub fn extract_u32(&mut self) -> Result<u32, CustomError> {
        let slice: [u8; 4] = self
            .extract_buffer(4)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(u32::from_le_bytes(slice))
    }
    pub fn extract_u64(&mut self) -> Result<u64, CustomError> {
        let slice: [u8; 8] = self
            .extract_buffer(8)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(u64::from_le_bytes(slice))
    }
    pub fn extract_i8(&mut self) -> Result<i8, CustomError> {
        let slice: [u8; 1] = self
            .extract_buffer(1)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(i8::from_le_bytes(slice))
    }
    pub fn extract_i16(&mut self) -> Result<i16, CustomError> {
        let slice: [u8; 2] = self
            .extract_buffer(2)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(i16::from_le_bytes(slice))
    }
    pub fn extract_i32(&mut self) -> Result<i32, CustomError> {
        let slice: [u8; 4] = self
            .extract_buffer(4)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(i32::from_le_bytes(slice))
    }
    pub fn extract_i64(&mut self) -> Result<i64, CustomError> {
        let slice: [u8; 8] = self
            .extract_buffer(8)?
            .try_into()
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

        Ok(i64::from_le_bytes(slice))
    }

    pub fn extract_varint(&mut self) -> Result<u64, CustomError> {
        let first_byte = self.extract_u8()?;
        let slice = match first_byte {
            0xFF_u8 => {
                let slice: [u8; 8] = self
                    .extract_buffer(8)?
                    .try_into()
                    .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

                u64::from_le_bytes([
                    slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
                ])
            }
            0xFE_u8 => {
                let slice: [u8; 4] = self
                    .extract_buffer(4)?
                    .try_into()
                    .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

                u64::from_le_bytes([slice[0], slice[1], slice[2], slice[3], 0, 0, 0, 0])
            }
            0xFD_u8 => {
                let slice: [u8; 2] = self
                    .extract_buffer(2)?
                    .try_into()
                    .map_err(|_| CustomError::SerializedBufferIsInvalid)?;

                u64::from_le_bytes([slice[0], slice[1], 0, 0, 0, 0, 0, 0])
            }
            _ => u64::from_le_bytes([first_byte, 0, 0, 0, 0, 0, 0, 0]),
        };
        Ok(slice)
    }

    pub fn extract_address(&mut self) -> Result<SocketAddrV6, CustomError> {
        let ipv6 = Ipv6Addr::new(
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
            u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]),
        );
        let port = u16::from_be_bytes([self.extract_u8()?, self.extract_u8()?]);
        let socket = SocketAddrV6::new(ipv6, port, 0, 0);
        Ok(socket)
    }
    pub fn extract_string(&mut self, size: usize) -> Result<String, CustomError> {
        let buffer = self.extract_buffer(size)?;
        let string = String::from_utf8(buffer.to_vec())
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;
        Ok(string)
    }
}

pub trait VarIntSerialize {
    fn to_varint_bytes(&self) -> Vec<u8>;
}

impl VarIntSerialize for usize {
    fn to_varint_bytes(&self) -> Vec<u8> {
        if *self < 0xFD {
            return (*self as u8).to_le_bytes().to_vec();
        }
        if *self <= 0xFFFF {
            let mut buffer = [0xFD_u8].to_vec();
            buffer.append(&mut (*self as u16).to_le_bytes().to_vec());
            return buffer;
        }
        if *self <= 0xFFFFFFFF {
            let mut buffer = [0xFE_u8].to_vec();
            buffer.append(&mut (*self as u32).to_le_bytes().to_vec());
            return buffer;
        }
        let mut buffer = [0xFF_u8].to_vec();
        buffer.append(&mut self.to_le_bytes().to_vec());
        buffer
    }
}
