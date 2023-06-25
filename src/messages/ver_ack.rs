use crate::{error::CustomError, message::Message};

#[derive(Debug)]
/// VerAck es un mensaje vacio que se envia tras intercambiar los mensajes de version.
/// Sirve para confirmar que se ha establecido la conexión.
pub struct VerAck {}

impl VerAck {
    /// Crea un nuevo mensaje de verificación de conexión.
    pub fn new() -> Self {
        VerAck {}
    }
}

impl Default for VerAck {
    fn default() -> Self {
        VerAck::new()
    }
}

/// Implementa el trait Message para el mensaje de verificación de conexión.
impl Message for VerAck {
    /// Devuelve el comando del mensaje.
    /// En este caso, el comando es "verack".
    fn get_command(&self) -> String {
        String::from("verack")
    }

    /// Devuelve un vector vacío.
    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

    /// Parsea un vector de bytes en un mensaje de verificación de conexión.
    /// Si el vector no está vacío, devuelve un CustomError.
    fn parse(buffer: Vec<u8>) -> Result<Self, crate::error::CustomError> {
        if !buffer.is_empty() {
            return Err(CustomError::SerializedBufferIsInvalid);
        }
        Ok(VerAck {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ver_ack_creation() {
        let verack = VerAck::new();
        let serialized_verack = verack.serialize();
        assert_eq!(serialized_verack, vec![]);

        let verack = VerAck::default();
        let serialized_verack = verack.serialize();
        assert_eq!(serialized_verack, vec![]);
    }

    #[test]
    fn serialize_verack() {
        let verack = VerAck::new();
        let serialized_verack = verack.serialize();
        assert_eq!(serialized_verack, vec![]);
    }

    #[test]
    fn parse_verack() {
        let verack = VerAck::new();
        let serialized_verack = verack.serialize();
        let parsed_verack = VerAck::parse(serialized_verack);
        assert_eq!(parsed_verack.is_ok(), true);
    }

    #[test]
    fn parse_invalid_verack() {
        let buffer_too_long = vec![0x00];
        let parsed_verack = VerAck::parse(buffer_too_long);
        assert_eq!(parsed_verack.is_err(), true);
    }

    #[test]
    fn get_command_verack() {
        let verack = VerAck::new();
        let command = verack.get_command();
        assert_eq!(command, String::from("verack"));
    }
}
