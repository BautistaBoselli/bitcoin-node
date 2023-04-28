use crate::error::CustomError;
use crate::message::{Message, MessageHeader};
use crate::messages::version::Version;
use std::net::TcpStream;
use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6, ToSocketAddrs},
    vec::IntoIter,
};

#[derive(Debug)]
pub struct Node {
    pub ip_v6: Ipv6Addr,
    pub services: u64,
    pub port: u16,
    pub version: i32,
    pub stream: Option<TcpStream>,
    pub handshake: bool,
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
    pub fn new(address: SocketAddr) -> Result<Self, CustomError> {
        let ip_v6 = match address {
            SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
            SocketAddr::V6(addr) => addr.ip().to_owned(),
        };

        let stream = TcpStream::connect(address).map_err(|_| CustomError::CannotConnectToNode)?;

        Ok(Self {
            ip_v6: ip_v6,
            services: 0,
            port: address.port(),
            version: 0,
            stream: Some(stream),
            handshake: false,
        })
    }
    pub fn handshake(&mut self, sender_node: &Node) -> Result<(), CustomError> {
        let version_message =
            Version::new(sender_node, SocketAddrV6::new(self.ip_v6, self.port, 0, 0));

        let stream = match &mut self.stream {
            Some(stream) => stream,
            None => return Err(CustomError::CannotHandshakeNode),
        };

        version_message.send(stream)?;

        let response_header = MessageHeader::read(stream)?;

        if response_header.command != "version".to_string() {
            return Err(CustomError::CannotHandshakeNode);
        }

        let version_response = Version::read(stream, response_header.payload_size)?;
        self.version = version_response.version;
        self.services = version_response.services;

        println!("Version: {:?}", version_response);

        // let response_header = MessageHeader::read(stream)?;

        // if response_header.command != "verack".to_string() {
        //     return Err(CustomError::CannotHandshakeNode);
        // }

        // let version_response = VerAck::read(stream)?;
        // self.version = version_response.version;
        // self.services = version_response.services;

        // println!("Version: {:?}", version_response);

        self.handshake = true;
        Ok(())
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
