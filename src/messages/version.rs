use std::net::{Ipv6Addr, SocketAddrV6};

use crate::error::CustomError;
use crate::message::Message;
use crate::node::Node;

#[derive(PartialEq, Debug)]
/// Crea una estructura para el mensaje de versión con los campos necesarios de acuerdo con el protocolo de Bitcoin.
/// Los campos son:
/// - version: que indica la versión del protocolo.
/// - services: que indica los servicios que ofrece el nodo.
/// - timestamp: que indica el timestamp del nodo que envía el mensaje.
/// - receiver_services: que indica los servicios que se espera que pueda ofrecer el nodo que recibe el mensaje.
/// - receiver_address: que indica la dirección IPv6 del nodo que recibe el mensaje.
/// - receiver_port: que indica el puerto del nodo que recibe el mensaje.
/// - sender_services: que indica los servicios que ofrece el nodo que envía el mensaje.
/// - sender_address: que indica la dirección IPv6 del nodo que envía el mensaje.
/// - sender_port: que indica el puerto del nodo que envía el mensaje.
/// - nonce: que indica un número aleatorio que se utiliza para detectar conexiones a sí mismo.
/// - user_agent: que indica el software que utiliza el nodo que envía el mensaje, puede ser vacío.
/// - user_agent_length: que indica la longitud del campo user_agent. Si es 0, el campo user_agent no se incluye.
/// - start_height: que indica el tamaño de la blockchain del nodo que envía el mensaje.
pub struct Version {
    pub version: i32,
    pub services: u64,
    pub timestamp: u64,
    pub receiver_services: u64,
    pub receiver_address: Ipv6Addr,
    pub receiver_port: u16,
    pub sender_services: u64,
    pub sender_address: Ipv6Addr,
    pub sender_port: u16,
    pub nonce: u64,
    pub user_agent: String,
    pub user_agent_length: u8,
    pub start_height: i32,
}

impl Version {
    /// Crea un nuevo mensaje de versión a partir de un nodo transmisor y una dirección IPv6 del nodo que recibe el mensaje.
    /// El campo user_agent se inicializa con un string vacío y el campo user_agent_length con 0.
    /// El campo nonce se inicializa con 0.
    /// El campo start_height se inicializa con 0.
    pub fn new(sender_node: &Node, receiver_address: SocketAddrV6) -> Self {
        Version {
            version: sender_node.version,
            services: sender_node.services,
            timestamp: chrono::Utc::now().timestamp() as u64,
            receiver_services: 0x00,
            receiver_address: *receiver_address.ip(),
            receiver_port: receiver_address.port(),
            sender_services: sender_node.services,
            sender_address: sender_node.ip_v6,
            sender_port: sender_node.port,
            nonce: 0x00,
            user_agent: String::from(""),
            user_agent_length: 0x00,
            start_height: 0x00,
        }
    }
}

/// Implementa el trait Message para el mensaje de versión.
impl Message for Version {
    /// Devuelve el comando del mensaje.
    /// En este caso, el comando es "version".
    fn get_command(&self) -> String {
        String::from("version")
    }

    /// Devuelve un mensaje de versión serializado en un vector de bytes.
    /// La mayoria de datos se envia en little endian, excepto las direcciones ip y puertos del nodo transmisor y receptor que se envian en big endian.
    fn serialize(&self) -> Vec<u8> {
        let mut buffer = vec![];
        buffer.extend_from_slice(&self.version.to_le_bytes());
        buffer.extend_from_slice(&self.services.to_le_bytes());
        buffer.extend_from_slice(&self.timestamp.to_le_bytes());
        buffer.extend_from_slice(&self.receiver_services.to_le_bytes());
        let ipv6_buffer = self.receiver_address.octets();
        for byte in ipv6_buffer {
            buffer.extend_from_slice(&[byte]);
        }
        buffer.extend_from_slice(&self.receiver_port.to_be_bytes());
        buffer.extend_from_slice(&self.sender_services.to_le_bytes());
        let ipv6_buffer = self.sender_address.octets();
        for byte in ipv6_buffer {
            buffer.extend_from_slice(&[byte]);
        }
        buffer.extend_from_slice(&self.sender_port.to_be_bytes());
        buffer.extend_from_slice(&self.nonce.to_le_bytes());
        buffer.extend_from_slice(&self.user_agent_length.to_le_bytes());
        buffer.extend_from_slice(self.user_agent.as_bytes());
        buffer.extend_from_slice(&self.start_height.to_le_bytes());

        buffer
    }

    /// Deserializa un vector de bytes en un mensaje de versión.
    /// Devuelve un CustomError si el vector de bytes no contiene la cantidad minima de bytes de un mensaje versión válido.
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        if buffer.len() < 85 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        let version = i32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let services = u64::from_le_bytes([
            buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10],
            buffer[11],
        ]);
        let timestamp = u64::from_le_bytes([
            buffer[12], buffer[13], buffer[14], buffer[15], buffer[16], buffer[17], buffer[18],
            buffer[19],
        ]);
        let receiver_services = u64::from_le_bytes([
            buffer[20], buffer[21], buffer[22], buffer[23], buffer[24], buffer[25], buffer[26],
            buffer[27],
        ]);
        let receiver_address = Ipv6Addr::new(
            u16::from_be_bytes([buffer[28], buffer[29]]),
            u16::from_be_bytes([buffer[30], buffer[31]]),
            u16::from_be_bytes([buffer[32], buffer[33]]),
            u16::from_be_bytes([buffer[34], buffer[35]]),
            u16::from_be_bytes([buffer[36], buffer[37]]),
            u16::from_be_bytes([buffer[38], buffer[39]]),
            u16::from_be_bytes([buffer[40], buffer[41]]),
            u16::from_be_bytes([buffer[42], buffer[43]]),
        );
        let receiver_port = u16::from_be_bytes([buffer[44], buffer[45]]);
        let sender_services = u64::from_le_bytes([
            buffer[46], buffer[47], buffer[48], buffer[49], buffer[50], buffer[51], buffer[52],
            buffer[53],
        ]);
        let sender_address = Ipv6Addr::new(
            u16::from_be_bytes([buffer[54], buffer[55]]),
            u16::from_be_bytes([buffer[56], buffer[57]]),
            u16::from_be_bytes([buffer[58], buffer[59]]),
            u16::from_be_bytes([buffer[60], buffer[61]]),
            u16::from_be_bytes([buffer[62], buffer[63]]),
            u16::from_be_bytes([buffer[64], buffer[65]]),
            u16::from_be_bytes([buffer[66], buffer[67]]),
            u16::from_be_bytes([buffer[68], buffer[69]]),
        );
        let sender_port = u16::from_be_bytes([buffer[70], buffer[71]]);
        let nonce = u64::from_le_bytes([
            buffer[72], buffer[73], buffer[74], buffer[75], buffer[76], buffer[77], buffer[78],
            buffer[79],
        ]);
        let user_agent_length = buffer[80];
        let user_agent = String::from_utf8(buffer[81..(81 + user_agent_length as usize)].to_vec())
            .map_err(|_| CustomError::SerializedBufferIsInvalid)?;
        let start_height = i32::from_le_bytes([
            buffer[81 + user_agent_length as usize],
            buffer[82 + user_agent_length as usize],
            buffer[83 + user_agent_length as usize],
            buffer[84 + user_agent_length as usize],
        ]);
        Ok(Version {
            version,
            services,
            timestamp,
            receiver_services,
            receiver_address,
            receiver_port,
            sender_services,
            sender_address,
            sender_port,
            nonce,
            user_agent,
            user_agent_length,
            start_height,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create_version_message() -> Result<(), CustomError> {
        let test_node = Node {
            ip_v6: Ipv6Addr::new(0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0),
            services: 0x00,
            port: 4321,
            version: 7000,
            peers: vec![],
        };

        let receiver_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
        let version = Version::new(&test_node, receiver_address);
        let buffer = version.serialize();
        let parsed_version = Version::parse(buffer)?;
        assert_eq!(version, parsed_version);
        Ok(())
    }

    #[test]
    fn parse_version() -> Result<(), CustomError> {
        let test_node = Node {
            ip_v6: Ipv6Addr::new(0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0),
            services: 0x00,
            port: 4321,
            version: 7000,
            peers: vec![],
        };

        let receiver_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
        let version = Version::new(&test_node, receiver_address);
        let buffer = version.serialize();
        let parsed_version = Version::parse(buffer)?;
        assert_eq!(version, parsed_version);
        Ok(())
    }

    #[test]
    fn parse_invalid_version() {
        let buffer_too_short = vec![
            127, 17, 1, 0, 9, 4, 0, 0, 0, 0, 0, 0, 48, 21, 75, 100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 40, 0, 0, 64, 0, 27, 8, 11, 68, 134, 135, 118, 52, 198, 86, 32, 213, 227, 9, 4,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let parsed_version = Version::parse(buffer_too_short);
        assert_eq!(parsed_version.is_err(), true);
    }
}
