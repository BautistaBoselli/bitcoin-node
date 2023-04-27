use crate::error::CustomError;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

use std::io::Read;
use std::net::TcpStream;
use std::{io::Write, net::Ipv6Addr};

pub trait Message {
    fn serialize(&self) -> Vec<u8>;
    fn get_address(&self) -> (Ipv6Addr, u16);
    fn get_command(&self) -> String;
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized;

    fn send(&self, stream: &mut TcpStream) -> Result<(), CustomError>
    where
        Self: Sized,
    {
        let header = MessageHeader::new(self);

        stream
            .write(&header.serialize())
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        stream
            .write(&self.serialize())
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        stream
            .flush()
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        Ok(())
    }

    fn read(stream: &mut TcpStream) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        let mut header_buffer = [0; 24];

        stream
            .read_exact(&mut header_buffer)
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        println!("Received header: {:?}", header_buffer);

        let header = MessageHeader::parse(header_buffer)?;
        println!("Received header: {:?}", header);

        let mut payload_buffer = vec![0; header.payload_size as usize];

        stream
            .read_exact(&mut payload_buffer)
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        Ok(Self::parse(payload_buffer)?)
    }
}

fn get_checksum(payload: &Vec<u8>) -> [u8; 4] {
    let hash = sha256::Hash::hash(sha256::Hash::hash(payload).as_byte_array());
    [hash[0], hash[1], hash[2], hash[3]]
}

const MAGIC: u32 = 0x0b110907;
#[derive(Debug)]
pub struct MessageHeader {
    magic: u32,
    pub command: String,
    pub payload_size: u32,
    checksum: [u8; 4],
}

impl MessageHeader {
    pub fn new(message: &dyn Message) -> Self {
        let payload = message.serialize();
        let payload_size = payload.len() as u32;
        let checksum = get_checksum(&payload);

        MessageHeader {
            magic: MAGIC,
            command: message.get_command(),
            payload_size,
            checksum,
        }
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut header = vec![0; 24];

        let mut command = self.command.as_bytes().to_vec();
        command.resize(12, 0);

        header[0..4].copy_from_slice(&self.magic.to_be_bytes());
        header[4..16].copy_from_slice(&command);
        header[16..20].copy_from_slice(&self.payload_size.to_le_bytes());
        header[20..24].copy_from_slice(&self.checksum);
        header
    }

    pub fn parse(buffer: [u8; 24]) -> Result<Self, CustomError> {
        if buffer.len() != 24 {
            return Err(CustomError::InvalidHeader);
        }
        let magic = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let command = String::from_utf8(buffer[4..16].to_vec())
            .unwrap()
            .replace("\0", "");
        let payload_size = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let checksum = [buffer[20], buffer[21], buffer[22], buffer[23]];

        Ok(MessageHeader {
            magic,
            command,
            payload_size,
            checksum,
        })
    }
    pub fn read(stream: &mut TcpStream) -> Result<Self, CustomError> {
        let mut header_buffer = [0; 24];

        stream
            .read_exact(&mut header_buffer)
            .map_err(|_| CustomError::CannotHandshakeNode)
            .unwrap();

        println!("Received header: {:?}", header_buffer);

        let header = Self::parse(header_buffer).unwrap();
        println!("Received header: {:?}", header);

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddrV6;

    use crate::{node::Node, version::Version};

    use super::*;

    #[test]
    fn test_get_checksum() {
        let payload = "payload".as_bytes().to_vec();
        let checksum = get_checksum(&payload);
        assert_eq!(checksum, [0xe7, 0x87, 0x31, 0xbb]);
    }

    #[test]
    fn test_message_header_length() {
        let test_node = Node {
            ipv6: Ipv6Addr::new(0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0),
            services: 0x00,
            port: 4321,
            version: 7000,
        };

        let receiver_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
        let version = Version::new(&test_node, receiver_address);

        let header = MessageHeader::new(&version).serialize();
        assert_eq!(header.len(), 24);
    }

    #[test]
    fn test_message_header() {
        let header = [
            11, 17, 9, 7, 118, 101, 114, 115, 105, 111, 110, 0, 0, 0, 0, 0, 85, 0, 0, 0, 75, 114,
            249, 186,
        ];

        let header = MessageHeader::parse(header).unwrap();

        assert_eq!(header.magic, MAGIC);
        assert_eq!(header.command, "version");
        assert_eq!(header.payload_size, (85 as u32));
        assert_eq!(header.checksum.len(), 4);
        assert_eq!(header.checksum, [75, 114, 249, 186]);
    }
}
