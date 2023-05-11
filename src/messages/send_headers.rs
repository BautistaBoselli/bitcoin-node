use crate::{error::CustomError, message::Message};

#[derive(Debug)]
/// SendHeaders es un mensaje vacio que se envia tras intercambiar los mensajes de version.
/// Sirve para confirmar que se ha establecido la conexión.
///
pub struct SendHeaders {}

impl SendHeaders {
    /// Crea un nuevo mensaje de verificación de conexión.
    pub fn new() -> Self {
        SendHeaders {}
    }
}

impl Default for SendHeaders {
    fn default() -> Self {
        SendHeaders::new()
    }
}

/// Implementa el trait Message para el mensaje de verificación de conexión.
impl Message for SendHeaders {
    /// Devuelve el comando del mensaje.
    /// En este caso, el comando es "SendHeaders".
    fn get_command(&self) -> String {
        String::from("sendheaders")
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
        Ok(SendHeaders {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_send_headers() {
        let send_headers = SendHeaders::new();
        let serialize_send_headers = send_headers.serialize();
        assert_eq!(serialize_send_headers, vec![]);
    }

    #[test]
    fn parse_send_headers() {
        let send_headers = SendHeaders::new();
        let serialize_send_headers = send_headers.serialize();
        let parsed_send_headers = SendHeaders::parse(serialize_send_headers);
        assert_eq!(parsed_send_headers.is_ok(), true);
    }

    #[test]
    fn parse_invalid_send_headers() {
        let buffer_too_long = vec![0x00];
        let parsed_send_headers = SendHeaders::parse(buffer_too_long);
        assert_eq!(parsed_send_headers.is_err(), true);
    }
}
