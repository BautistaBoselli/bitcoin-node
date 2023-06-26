use crate::{error::CustomError, message::Message};

#[derive(Debug)]
/// SendHeaders es un mensaje vacio que se envia tras intercambiar los mensajes de version.
/// Sirve para confirmar que se ha establecido la conexión y para recibir los headers de los bloques directamente.
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
/// Permite serializar, parsear y obtener el comando
impl Message for SendHeaders {
    fn get_command(&self) -> String {
        String::from("sendheaders")
    }

    fn serialize(&self) -> Vec<u8> {
        vec![]
    }

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

    #[test]
    fn get_command_send_headers() {
        let send_headers = SendHeaders::new();
        assert_eq!(send_headers.get_command(), String::from("sendheaders"));
    }
}
