use std::{
    io::Read,
    net::{SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs},
    sync::{mpsc, Arc, Mutex},
    thread,
    vec::IntoIter,
};

use crate::{
    error::CustomError,
    message::{Message, MessageHeader},
    messages::{get_headers::GetHeaders, ver_ack::VerAck, version::Version},
    // peer::Peer,
};

pub fn get_addresses(seed: String, port: u16) -> Result<IntoIter<SocketAddr>, CustomError> {
    (seed, port)
        .to_socket_addrs()
        .map_err(|_| CustomError::CannotResolveSeedAddress)
}

pub enum PeerAction {
    GetHeaders,
    GetBlock(String),
    Terminate,
}

pub enum PeerResponse {
    NewHeaders(String),
    GetHeadersError,
    Block((String, String)),
    GetBlockError(String),
}

pub struct Peer {
    pub id: SocketAddr,
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    pub stream: TcpStream,
    pub node_listener_thread: Option<thread::JoinHandle<()>>,
    pub stream_listener_thread: Option<thread::JoinHandle<()>>,
    pub logger_sender: mpsc::Sender<String>,
}

impl Peer {
    pub fn new(
        address: SocketAddr,
        sender_address: SocketAddrV6,
        services: u64,
        version: i32,
        receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        logger_sender: mpsc::Sender<String>,
        peers_response_sender: mpsc::Sender<PeerResponse>,
    ) -> Result<Self, CustomError> {
        let ip_v6 = match address {
            SocketAddr::V4(addr) => addr.ip().to_ipv6_mapped(),
            SocketAddr::V6(addr) => addr.ip().to_owned(),
        };

        let stream = TcpStream::connect(address).map_err(|_| CustomError::CannotConnectToNode)?;

        let mut peer = Self {
            id: address,
            address: SocketAddrV6::new(ip_v6, address.port(), 0, 0),
            node_listener_thread: None,
            stream_listener_thread: None,
            services,
            version,
            stream,
            logger_sender,
        };

        peer.handshake(sender_address)?;

        let peer_response_sender_clone = peers_response_sender.clone();
        let logger_sender_clone = peer.logger_sender.clone();
        let mut thread_stream = peer.stream.try_clone().unwrap();

        let node_listener_thread = thread::spawn(move || loop {
            let peer_message = receiver.lock().unwrap().recv().unwrap();
            match peer_message {
                PeerAction::GetHeaders => {
                    println!("Recibido el pedido de headers...");
                    let request = GetHeaders::new(version, vec![0; 32]).send(&mut thread_stream);

                    if request.is_err() {
                        logger_sender_clone
                            .send("Error pidiendo headers".to_string())
                            .unwrap();
                        peer_response_sender_clone
                            .send(PeerResponse::GetHeadersError)
                            .unwrap();
                    }
                }
                PeerAction::GetBlock(block_header) => {
                    println!("Recibido el pedido de bloque {}...", block_header);
                    return;
                }
                PeerAction::Terminate => {
                    break;
                }
            }
        });

        let peer_response_sender_clone = peers_response_sender.clone();
        let logger_sender_clone = peer.logger_sender.clone();
        let mut thread_stream = peer.stream.try_clone().unwrap();

        let stream_listener_thread = thread::spawn(move || loop {
            let response_header = match MessageHeader::read(&mut thread_stream) {
                Ok(header) => header,
                Err(error) => {
                    println!("Error al leer el header: {}", error);
                    return;
                }
            };

            if response_header.command.as_str() == "headers" {
                println!("Recibida la respuesta de headers...");
                let _response =
                    match GetHeaders::read(&mut thread_stream, response_header.payload_size) {
                        Ok(response) => response,
                        Err(error) => {
                            println!("Error al leer el mensaje: {}", error);
                            return;
                        }
                    };

                peer_response_sender_clone
                    .send(PeerResponse::NewHeaders("NUEVOOOSS HEADEARSS".to_string()))
                    .unwrap();
            } else {
                let cmd = response_header.command.as_str();

                if cmd != "ping" && cmd != "alert" && cmd != "addr" {
                    logger_sender_clone
                        .send(format!(
                            "Recibido desconocido: {:?}",
                            response_header.command
                        ))
                        .unwrap();
                }
                let mut buffer: Vec<u8> = vec![0; response_header.payload_size as usize];
                thread_stream.read_exact(&mut buffer).unwrap();
            }
        });

        peer.node_listener_thread = Some(node_listener_thread);
        peer.stream_listener_thread = Some(stream_listener_thread);
        Ok(peer)
    }

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
}
