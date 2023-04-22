use std::{net::{ToSocketAddrs, SocketAddr, IpAddr, Ipv6Addr}};
use std::vec::IntoIter;

use crate::{config::Config, error::CustomError};

struct Version {
    version: i32,
    services: u64,
    timestamp: u64,
    receiver_services: u64,
    receiver_address: SocketAddr,
    receiver_port: u16,
    sender_services: u64,
    sender_address: SocketAddr,
    sender_port: u16,
    nonce: u64,
    user_agent: String,
    user_agent_length: u8,
    start_height: i32,
}

struct Peer {
    address: SocketAddr,
    services: u64,
    port: u16,
}
    
impl Version {
    pub fn new(config: Config, peer: Peer) -> Self {
        Version{
            version: config.protocol_version as i32,
            services: 0x00,
            timestamp: chrono::Utc::now().timestamp() as u64,
            receiver_services: peer.services,
            receiver_address: peer.address,
            receiver_port: peer.port,
            sender_services: 0x00,
            sender_address: SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xf,0xf,0xf,0xf,0, 0, 0, 0)), 0),
            sender_port: config.port,
            nonce: 0x00,
            user_agent: String::from(""),
            user_agent_length: 0x00,
            start_height: 0x00,

        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buffer = vec![];
        buffer.extend_from_slice(&self.version.to_be_bytes());
        buffer.extend_from_slice(&self.services.to_be_bytes());
        buffer.extend_from_slice(&self.timestamp.to_be_bytes());
        buffer.extend_from_slice(&self.receiver_services.to_be_bytes());
        buffer.extend_from_slice(&self.receiver_address.ip().octets().to_be_bytes());
        buffer.extend_from_slice(&self.receiver_port.to_be_bytes());
        buffer.extend_from_slice(&self.sender_services.to_be_bytes());
        buffer.extend_from_slice(&self.sender_address.to_be_bytes());
        buffer.extend_from_slice(&self.sender_port.to_be_bytes());
        buffer.extend_from_slice(&self.nonce.to_be_bytes());
        buffer.extend_from_slice(&self.user_agent_length.to_be_bytes());
        buffer.extend_from_slice(&self.user_agent.as_bytes());
        buffer.extend_from_slice(&self.start_height.to_be_bytes());
        buffer
    }
}

/// Conecta con la semilla DNS y devuelve un iterador de direcciones IP.
/// Devuelve CustomError si:
/// - No se pudo resolver la semilla DNS.
pub fn connect(config: Config) -> Result<IntoIter<SocketAddr>, CustomError>{
    (config.seed, config.port).to_socket_addrs().map_err(|_| CustomError::CannotResolveSeedAddress)
}



#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn connect_to_seed_invalida() {
        let config = Config{
            seed: String::from("seed.test"),
            protocol_version: 7000,
            port: 4321,
        };
        let addresses = connect(config);
        assert!(addresses.is_err());
    }

    #[test]
    fn connect_to_seed_valida() -> Result<(), CustomError> {
        let config = Config{
            seed: String::from("google.com"),
            protocol_version: 7000,
            port: 80,
        };
        let addresses = connect(config)?;
        assert!(addresses.len() > 0);
        Ok(())
    }
 }