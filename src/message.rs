use crate::error::CustomError;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

use std::{
    io::{Read, Write},
    net::Ipv6Addr,
};

pub trait Message {
    fn serialize(&self) -> Vec<u8>;
    fn get_address(&self) -> (Ipv6Addr, u16);
    fn get_command(&self) -> String;
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized;
}

pub trait Test {
    fn test(&self) -> String;
}

pub fn send(message: &dyn Message) -> Result<Vec<u8>, CustomError> {
    let mut stream = std::net::TcpStream::connect(message.get_address())
        .map_err(|_| CustomError::CannotConnectToNode)?;

    let header = get_message_header(message);
    let buffer = message.serialize();
    let content = [&header[..], &buffer[..]].concat();

    stream
        .write(&content)
        .map_err(|_| CustomError::CannotHandshakeNode)?;

    stream
        .flush()
        .map_err(|_| CustomError::CannotHandshakeNode)?;
    println!("Sent: {:?}", content);
    println!("Sent {:?} bytes", content.len());

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|_| CustomError::CannotHandshakeNode)?;
    println!("Received: {:?}", response);
    Ok(response)
}

static MAGIC: u32 = 0xD9B4BEF9;
fn get_message_header(message: &dyn Message) -> Vec<u8> {
    let mut header = vec![0; 24];
    let magic = MAGIC.to_be_bytes();
    let mut command = message.get_command().as_bytes().to_vec();
    let payload = message.serialize();
    let payload_size = payload.len() as u32;
    let checksum = get_checksum(&payload);

    command.resize(12, 0);

    header[0..4].copy_from_slice(&magic);
    header[4..16].copy_from_slice(&command);
    header[16..20].copy_from_slice(&payload_size.to_be_bytes());
    header[20..24].copy_from_slice(&checksum);
    header
}

fn get_checksum(payload: &Vec<u8>) -> [u8; 4] {
    let hash = sha256::Hash::hash(sha256::Hash::hash(payload).as_byte_array());
    [hash[0], hash[1], hash[2], hash[3]]
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
        let version = Version::new(test_node, receiver_address);

        let header = get_message_header(&version);
        assert_eq!(header.len(), 24);
    }
}
