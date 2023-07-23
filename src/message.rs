use crate::error::CustomError;
use bitcoin_hashes::sha256;
use bitcoin_hashes::Hash;

use std::io::Read;
use std::io::Write;
use std::net::TcpStream;

/// Este trait representa un mensaje del protocolo.
/// Todos los mensajes deben implementar este trait, por lo que todos deben poder:
/// - Serializarse.
/// - Parsearse.
/// - Obtener su comando.
/// Esto se realiza de forma distinta para cada mensaje, por lo que se implementa de forma individual en cada uno.
/// A parte de esto, todos los mensajes deben poder enviarse a un stream y leerse de un stream.
/// Para ello, se implementan los métodos send y read que al ser el procedimiento igual en todos los mensajes, no requieren implementación individual.
pub trait Message {
    fn serialize(&self) -> Vec<u8>;
    fn get_command(&self) -> String;
    fn parse(buffer: Vec<u8>) -> Result<Self, CustomError>
    where
        Self: Sized;

    /// Envía el mensaje a un stream.
    /// Devuelve CustomError si:
    /// - No se puede escribir en el stream el header del mensaje.
    /// - No se puede escribir en el stream el payload del mensaje.
    /// - No se puede hacer flush del stream.
    fn send(&self, stream: &mut TcpStream) -> Result<(), CustomError>
    where
        Self: Sized,
    {
        let header = MessageHeader::new(self);

        stream
            .write(&header.serialize())
            .map_err(|_| CustomError::CannotSendMessageToChannel)?;

        stream
            .write(&self.serialize())
            .map_err(|_| CustomError::CannotSendMessageToChannel)?;

        stream
            .flush()
            .map_err(|_| CustomError::CannotSendMessageToChannel)?;

        Ok(())
    }

    /// Lee un mensaje de un stream y lo parsea.
    /// Devuelve CustomError si:
    /// - No se puede leer del stream
    fn read(stream: &mut TcpStream, message_size: u32) -> Result<Self, CustomError>
    where
        Self: Sized,
    {
        let mut payload_buffer = vec![0; message_size as usize];

        stream
            .read_exact(&mut payload_buffer)
            .map_err(|_| CustomError::CannotReadStream)?;

        Self::parse(payload_buffer)
    }
}

/// Calcula el checksum de un payload.
/// El checksum es el hash de doble aplicación de sha256.
/// Devuelve los primeros 4 bytes del hash.
fn get_checksum(payload: &[u8]) -> [u8; 4] {
    let hash = sha256::Hash::hash(sha256::Hash::hash(payload).as_byte_array());
    [hash[0], hash[1], hash[2], hash[3]]
}

/// El magic number es un número que se usa para identificar la red, en nuestro caso, la testnet.
const MAGIC: u32 = 0x0b110907;
#[derive(Debug)]
/// Representa el header de un mensaje.
/// El header contiene:
/// - Un magic number que identifica la red.
/// - Un comando que identifica el tipo de mensaje.
/// - El tamaño del payload.
/// - El checksum del payload.
pub struct MessageHeader {
    magic: u32,
    pub command: String,
    pub payload_size: u32,
    checksum: [u8; 4],
}

impl MessageHeader {
    /// Crea un nuevo header a partir de un mensaje.
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

    /// Serializa el header que siempre tiene un tamaño de 24 bytes.
    /// Los campos se serializan en el siguiente orden:
    /// - Magic number: 4 bytes.
    /// - Command: 12 bytes.
    /// - Payload size: 4 bytes.
    /// - Checksum: 4 bytes.
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

    /// Parsea un header a partir de un buffer de 24 bytes.
    /// Devuelve CustomError si:
    /// - El buffer no tiene 24 bytes.
    /// - El comando no se puede parsear a String.
    pub fn parse(buffer: [u8; 24]) -> Result<Self, CustomError> {
        if buffer.len() != 24 {
            return Err(CustomError::CannotReadMessageHeader);
        }
        let magic = u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let mut command = match String::from_utf8(buffer[4..16].to_vec()) {
            Ok(command) => command,
            Err(_) => {
                return Err(CustomError::CannotReadMessageHeader);
            }
        };
        command = command.replace('\0', "");
        let payload_size = u32::from_le_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]);
        let checksum = [buffer[20], buffer[21], buffer[22], buffer[23]];

        Ok(MessageHeader {
            magic,
            command,
            payload_size,
            checksum,
        })
    }

    /// Lee un header de un stream y lo parsea.
    /// Devuelve CustomError si:
    /// - No se puede leer del stream.
    pub fn read(stream: &mut TcpStream) -> Result<Self, CustomError> {
        let mut header_buffer = [0; 24];

        stream
            .read_exact(&mut header_buffer)
            .map_err(|_| CustomError::CannotReadMessageHeader)?;

        let header = Self::parse(header_buffer)?;

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddrV6};

    use crate::messages::version::Version;

    use super::*;

    #[test]
    fn test_get_checksum() {
        let payload = "payload".as_bytes().to_vec();
        let checksum = get_checksum(&payload);
        assert_eq!(checksum, [0xe7, 0x87, 0x31, 0xbb]);
    }

    #[test]
    fn test_message_header_length() {
        let sender_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 4321, 0, 0);
        let receiver_address = SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), 8080, 0, 0);
        let version = Version::new(receiver_address, sender_address, 70000, 0x00);

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
