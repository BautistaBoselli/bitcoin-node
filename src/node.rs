use crate::message::{Message, MessageHeader};
use crate::version::Version;
use crate::{error::CustomError, version};
use std::io::Read;
use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6, ToSocketAddrs},
    vec::IntoIter,
};

#[derive(Debug)]
pub struct Node {
    pub ipv6: Ipv6Addr,
    pub services: u64,
    pub port: u16,
    pub version: i32,
}

/// Conecta con la semilla DNS y devuelve un iterador de direcciones IP.
/// Devuelve CustomError si:
/// - No se pudo resolver la semilla DNS.
pub fn get_addresses(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError> {
    (seed, port)
        .to_socket_addrs()
        .map_err(|_| CustomError::CannotResolveSeedAddress)
}

impl Node {
    pub fn from_address(sender_node: &Node, address: SocketAddrV6) -> Result<Self, CustomError> {
        let version_message = version::Version::new(sender_node, address);

        let mut stream = std::net::TcpStream::connect(version_message.get_address())
            .map_err(|_| CustomError::CannotConnectToNode)?;

        version_message.send(&mut stream)?;

        let response_header = MessageHeader::read(&mut stream)?;

        if response_header.command == "version".to_string() {
            let mut message_buffer = vec![0; response_header.payload_size as usize];
            stream
                .read_exact(&mut message_buffer)
                .map_err(|_| CustomError::InvalidHeader)?;

            let version = Version::parse(message_buffer)?;

            println!("Version: {:?}", version);

            return Ok(Node {
                ipv6: *address.ip(),
                services: version.services,
                port: address.port(),
                version: version.version,
            });
        }

        Err(CustomError::CannotConnectToNode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_to_seed_invalida() {
        let addresses = get_addresses(String::from("seed.test"), 4321);
        assert!(addresses.is_err());
    }

    #[test]
    fn connect_to_seed_valida() -> Result<(), CustomError> {
        let addresses = get_addresses(String::from("google.com"), 80)?;
        assert!(addresses.len() > 0);
        Ok(())
    }
}
