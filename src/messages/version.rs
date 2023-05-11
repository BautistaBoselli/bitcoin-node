use std::net::SocketAddrV6;

use crate::error::CustomError;
use crate::message::Message;
use crate::parser::BufferParser;

#[derive(PartialEq, Debug)]
/// Crea una estructura para el mensaje de versión con los campos necesarios de acuerdo con el protocolo de Bitcoin.
/// Los campos son:
/// - version: que indica la versión del protocolo.
/// - services: que indica los servicios que ofrece el nodo.
/// - timestamp: que indica el timestamp del nodo que envía el mensaje.
/// - receiver_services: que indica los servicios que se espera que pueda ofrecer el nodo que recibe el mensaje.
/// - receiver_address: que indica el socket v6 del nodo que recibe el mensaje.
/// - sender_services: que indica los servicios que ofrece el nodo que envía el mensaje.
/// - sender_address: que indica la socket v6 del nodo que envía el mensaje.
/// - nonce: que indica un número aleatorio que se utiliza para detectar conexiones a sí mismo.
/// - user_agent: que indica el software que utiliza el nodo que envía el mensaje, puede ser vacío.
/// - user_agent_length: que indica la longitud del campo user_agent. Si es 0, el campo user_agent no se incluye.
/// - start_height: que indica el tamaño de la blockchain del nodo que envía el mensaje.
pub struct Version {
    pub version: i32,
    pub services: u64,
    pub timestamp: u64,
    pub receiver_services: u64,
    pub receiver_address: SocketAddrV6,
    pub sender_services: u64,
    pub sender_address: SocketAddrV6,
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
    pub fn new(
        receiver_address: SocketAddrV6,
        sender_address: SocketAddrV6,
        version: i32,
        services: u64,
    ) -> Self {
        Version {
            version,
            services,
            timestamp: chrono::Utc::now().timestamp() as u64,
            receiver_services: 0x00,
            receiver_address,
            sender_services: services,
            sender_address,
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
        let ipv6_buffer = self.receiver_address.ip().octets();
        for byte in ipv6_buffer {
            buffer.extend_from_slice(&[byte]);
        }
        buffer.extend_from_slice(&self.receiver_address.port().to_be_bytes());
        buffer.extend_from_slice(&self.sender_services.to_le_bytes());
        let ipv6_buffer = self.sender_address.ip().octets();
        for byte in ipv6_buffer {
            buffer.extend_from_slice(&[byte]);
        }
        buffer.extend_from_slice(&self.sender_address.port().to_be_bytes());
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
        let mut parser = BufferParser::new(buffer);
        if parser.len() < 85 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        let version = parser.extract_i32()?;
        let services = parser.extract_u64()?;
        let timestamp = parser.extract_u64()?;
        let receiver_services = parser.extract_u64()?;
        let receiver_address = parser.extract_address()?;
        let sender_services = parser.extract_u64()?;
        let sender_address = parser.extract_address()?;
        let nonce = parser.extract_u64()?;
        let user_agent_length = parser.extract_u8()?;
        let user_agent = parser.extract_string(user_agent_length as usize)?;
        let start_height = parser.extract_i32()?;

        Ok(Version {
            version,
            services,
            timestamp,
            receiver_services,
            receiver_address,
            sender_services,
            sender_address,
            nonce,
            user_agent,
            user_agent_length,
            start_height,
        })
    }
}

#[cfg(test)]
mod tests {

    use std::net::Ipv6Addr;

    use super::*;

    #[test]
    fn create_version_message() -> Result<(), CustomError> {
        let sender_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 4321, 0, 0);
        let receiver_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 1234, 0, 0);
        let version: Version = Version::new(receiver_address, sender_address, 7000, 0x00);
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
