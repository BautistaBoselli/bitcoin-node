use std::net::{ToSocketAddrs, SocketAddr, IpAddr};
use std::vec::IntoIter;

use crate::error::CustomError;

struct Version {
    version: i32,
    services: u64,
    timestamp: u64,
    receiver_services: u64,
    receiver_address: IpAddr,
    receiver_port: u16,
    sender_services: u64,
    sender_address: IpAddr,
    sender_port: u16,
    nonce: u64,
    user_agent: String,
    user_agent_length: u8,
    start_height: i32,
}

pub struct Node {
    pub ipv6: IpAddr,
    pub services: u64,
    pub port: u16,
    pub version: i32,
}
    
impl Version {
    pub fn new(sender_node: Node, receiver_node: Node) -> Self {
        Version{
            version: sender_node.version,
            services: 0x00,
            timestamp: chrono::Utc::now().timestamp() as u64,
            receiver_services: receiver_node.services,
            receiver_address: receiver_node.ipv6,
            receiver_port: receiver_node.port,
            sender_services: 0x00,
            sender_address: sender_node.ipv6,
            sender_port: sender_node.port,
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
        match self.receiver_address {
            IpAddr::V6(receiver_ipv6) => {
                let ipv6_buffer = receiver_ipv6.octets();
                for byte in ipv6_buffer {
                    buffer.extend_from_slice(&[byte]);
                } 
            }
            _ => return vec![],
        }
        //buffer.extend_from_slice(SocketAddr::V6(self.receiver_address.).octets().to_be_bytes());
        buffer.extend_from_slice(&self.receiver_port.to_be_bytes());
        buffer.extend_from_slice(&self.sender_services.to_be_bytes());
        match self.sender_address {
            IpAddr::V6(sender_ipv6) => {
                let ipv6_buffer = sender_ipv6.octets();
                for byte in ipv6_buffer {
                    buffer.extend_from_slice(&[byte]);
                } 
            }
            _ => return vec![],
        }
        //buffer.extend_from_slice(&self.sender_address.to_be_bytes());
        buffer.extend_from_slice(&self.sender_port.to_be_bytes());
        buffer.extend_from_slice(&self.nonce.to_be_bytes());
        buffer.extend_from_slice(&self.user_agent_length.to_be_bytes());
        buffer.extend_from_slice(&self.user_agent.as_bytes());
        buffer.extend_from_slice(&self.start_height.to_be_bytes());
        buffer
    }

    // fn parse_version_message(buffer: &[u8]) -> Self {
    //     let version = i32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
    //     let services = u64::from_be_bytes([buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11]]);
    //     let timestamp = u64::from_be_bytes([buffer[12], buffer[13], buffer[14], buffer[15], buffer[16], buffer[17], buffer[18], buffer[19]]);
    //     let receiver_services = u64::from_be_bytes([buffer[20], buffer[21], buffer[22], buffer[23], buffer[24], buffer[25], buffer[26], buffer[27]]);
    //     let receiver_address = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(buffer[28], buffer[29], buffer[30], buffer[31], buffer[32], buffer[33], buffer[34], buffer[35])), u16::from_be_bytes([buffer[36], buffer[37]]));
    //     let receiver_port = u16::from_be_bytes([buffer[38], buffer[39]]);
    //     let sender_services = u64::from_be_bytes([buffer[40], buffer[41], buffer[42], buffer[43], buffer[44], buffer[45], buffer[46], buffer[47]]);
    //     let sender_address = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(buffer[48], buffer[49], buffer[50], buffer[51], buffer[52], buffer[53], buffer[54], buffer[55])), u16::from_be_bytes([buffer[56], buffer[57]]));
    //     let sender_port = u16::from_be_bytes([buffer[58], buffer[59]]);
    //     let nonce = u64::from_be_bytes([buffer[60], buffer[61], buffer[62], buffer[63], buffer[64], buffer[65], buffer[66], buffer[67]]);
    //     let user_agent_length = buffer[68];
    //     let user_agent = String::from_utf8(buffer[69..69+user_agent_length as usize].to_vec()).unwrap();
    //     let start_height = i32::from_be_bytes([buffer[69+user_agent_length as usize], buffer[70+user_agent_length as usize]]);
    // }
}

/// Conecta con la semilla DNS y devuelve un iterador de direcciones IP.
/// Devuelve CustomError si:
/// - No se pudo resolver la semilla DNS.
pub fn connect(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError>{
    (seed, port).to_socket_addrs().map_err(|_| CustomError::CannotResolveSeedAddress)
}



#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn connect_to_seed_invalida() {
        let addresses = connect(String::from("seed.test"), 4321);
        assert!(addresses.is_err());
    }
    

    #[test]
    fn connect_to_seed_valida() -> Result<(), CustomError> {
        let addresses = connect(String::from("google.com"), 80)?;
        assert!(addresses.len() > 0);
        Ok(())
    }
 }