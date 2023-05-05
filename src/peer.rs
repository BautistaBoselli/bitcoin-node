use crate::error::CustomError;
use crate::message::{Message, MessageHeader};
use crate::messages::ver_ack::VerAck;
use crate::messages::version::Version;
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::{
    net::{SocketAddr, SocketAddrV6, ToSocketAddrs},
    vec::IntoIter,
};

#[derive(Debug)]
/// Representa un peer de la red.
/// Contiene la dirección IPv6, los servicios que ofrece, el puerto, la versión del protocolo, el stream y el estado del handshake.
pub struct Peer {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    pub stream: TcpStream,
    logger_sender: Sender<String>,
}

/// Conecta con la semilla DNS y devuelve un iterador de direcciones IP.
/// Tanto la semilla como el puerto son parámetros recibidos del archivo de configuración.
/// Devuelve CustomError si:
/// - No se pudo resolver la semilla DNS.
pub fn get_addresses(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError> {
    (seed, port)
        .to_socket_addrs()
        .map_err(|_| CustomError::CannotResolveSeedAddress)
}

impl Peer {
    /// Crea un nuevo nodo a partir de un SocketAddr.
    /// Si el SocketAddr es IPv4, se convierte a IPv6, sino se obtiene la dirección IPv6.
    /// Crear un nuevo TcpStream y se intenta conectar al nodo cuya dirección se recibe como parámetro.
    /// Devuelve un nuevo nodo con el campo de stream inicializado al TcpStream creado y handshake en false.
    /// Devuelve CustomError si:
    /// - No se pudo crear el TcpStream.
    pub fn new(
        address: SocketAddr,
        services: u64,
        version: i32,
        logger_sender: Sender<String>,
    ) -> Result<Self, CustomError> {
        let stream = TcpStream::connect(address).map_err(|_| CustomError::CannotConnectToNode)?;

        let ip_v6 = match address {
            SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
            SocketAddr::V6(addr) => addr.ip().to_owned(),
        };

        let new_peer = Self {
            address: SocketAddrV6::new(ip_v6, address.port(), 0, 0),
            services,
            version,
            stream,
            logger_sender,
        };

        Ok(new_peer)
    }

    /// Genera el handshake entre &self (que es un nodo) y el otro nodo recibido como parámetro.
    /// Devuelve CustomError si:
    /// No existe un stream para el nodo.
    /// No se pudo enviar el mensaje de tipo Version.
    /// No se pudo leer el mensaje de respuesta.
    /// El primer mensaje de respuesta no es de tipo Version.
    /// No se pudo leer el mensaje de tipo VerAck.
    /// El segundo mensaje de respuesta no es de tipo VerAck.
    pub fn handshake(&mut self, sender_address: SocketAddrV6) -> Result<(), CustomError> {
        self.share_versions(sender_address)?;
        self.share_veracks()?;

        self.logger_sender
            .send(format!("Successful handshake with {}", self.address.ip()))
            .unwrap();

        Ok(())
    }

    fn share_versions(&mut self, sender_address: SocketAddrV6) -> Result<(), CustomError> {
        let version_message =
            Version::new(self.address, sender_address, self.version, self.services);
        version_message.send(&mut self.stream)?;

        let response_header = MessageHeader::read(&mut self.stream)?;
        if response_header.command.as_str() != "version" {
            return Err(CustomError::CannotHandshakeNode);
        }

        let version_response = Version::read(&mut self.stream, response_header.payload_size)?;
        self.version = version_response.version;
        self.services = version_response.services;

        Ok(())
    }

    fn share_veracks(&mut self) -> Result<(), CustomError> {
        let response_header = MessageHeader::read(&mut self.stream)?;
        if response_header.command.as_str() != "verack" {
            return Err(CustomError::CannotHandshakeNode);
        }
        VerAck::read(&mut self.stream, response_header.payload_size)?;
        let verack_message = VerAck::new();
        verack_message.send(&mut self.stream)?;
        Ok(())
    }

    pub fn get_headers(&self) -> Result<String, CustomError> {
        // loop para enviar los mensajes y conseguir los headers
        Ok(format!(
            "<<< NUEVOS HEADERS DESDE EL PEER {} >>>",
            self.address.ip()
        ))
    }

    pub fn get_block(&self, block_header: &String) -> Result<String, CustomError> {
        // loop para enviar los mensajes y conseguir los headers
        Ok(format!(
            "<<< BLOQUE {} DESDE EL PEER {} >>>",
            block_header,
            self.address.ip()
        ))
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
