use crate::error::CustomError;
use crate::message::{Message, MessageHeader};
use crate::messages::ver_ack::VerAck;
use crate::messages::version::Version;
use std::net::TcpStream;
use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6, ToSocketAddrs},
    vec::IntoIter,
};

#[derive(Debug)]
/// Representa un nodo de la red.
/// Contiene la dirección IPv6, los servicios que ofrece, el puerto, la versión del protocolo, el stream y el estado del handshake.
/// El stream es un campo opcional, ya que puede ser None si el nodo no está conectado.
/// Cuando es nuestra propia instancia de nodo, el stream es None.
/// El estado del handshake es un booleano que indica si se realizó el handshake con el nodo.
pub struct Node {
    pub ip_v6: Ipv6Addr,
    pub services: u64,
    pub port: u16,
    pub version: i32,
    pub stream: Option<TcpStream>,
    pub handshake: bool,
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

impl Node {
    /// Crea un nuevo nodo a partir de un SocketAddr.
    /// Si el SocketAddr es IPv4, se convierte a IPv6, sino se obtiene la dirección IPv6.
    /// Crear un nuevo TcpStream y se intenta conectar al nodo cuya dirección se recibe como parámetro.
    /// Devuelve un nuevo nodo con el campo de stream inicializado al TcpStream creado y handshake en false.
    /// Devuelve CustomError si:
    /// - No se pudo crear el TcpStream.
    pub fn new(address: SocketAddr) -> Result<Self, CustomError> {
        let ip_v6 = match address {
            SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
            SocketAddr::V6(addr) => addr.ip().to_owned(),
        };

        let stream = TcpStream::connect(address).map_err(|_| CustomError::CannotConnectToNode)?;

        Ok(Self {
            ip_v6,
            services: 0,
            port: address.port(),
            version: 0,
            stream: Some(stream),
            handshake: false,
        })
    }

    /// Genera el handshake entre &self (que es un nodo) y el otro nodo recibido como parámetro.
    /// Devuelve CustomError si:
    /// No existe un stream para el nodo.
    /// No se pudo enviar el mensaje de tipo Version.
    /// No se pudo leer el mensaje de respuesta.
    /// El primer mensaje de respuesta no es de tipo Version.
    /// No se pudo leer el mensaje de tipo VerAck.
    /// El segundo mensaje de respuesta no es de tipo VerAck.
    pub fn handshake(&mut self, sender_node: &Node) -> Result<(), CustomError> {
        let version_message =
            Version::new(sender_node, SocketAddrV6::new(self.ip_v6, self.port, 0, 0));

        let stream = match &mut self.stream {
            Some(stream) => stream,
            None => return Err(CustomError::CannotHandshakeNode),
        };

        version_message.send(stream)?;

        let response_header = MessageHeader::read(stream)?;

        if response_header.command.as_str() != "version" {
            return Err(CustomError::CannotHandshakeNode);
        }

        let version_response = Version::read(stream, response_header.payload_size)?;
        self.version = version_response.version;
        self.services = version_response.services;

        println!("Version: {:?}", version_response);

        let response_header = MessageHeader::read(stream)?;

        if response_header.command.as_str() != "verack" {
            return Err(CustomError::CannotHandshakeNode);
        }

        let ver_ack_response = VerAck::read(stream, response_header.payload_size)?;
        println!("VerAck: {:?}", ver_ack_response);

        //Puede ser que falte enviar nosotros el verack??

        self.handshake = true;
        Ok(())
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
