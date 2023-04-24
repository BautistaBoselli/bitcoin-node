use crate::message::Message;
use crate::{error::CustomError, version};
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
    pub fn from_address(sender_node: Node, address: SocketAddrV6) -> Result<Self, CustomError> {
        let version_message = version::Version::new(sender_node, address);

        let response = version_message.send()?;
        let parsed_response = version::Version::parse(response);

        Ok(Node {
            ipv6: *address.ip(),
            services: 0x00,
            port: address.port(),
            version: version_message.version,
        })
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
