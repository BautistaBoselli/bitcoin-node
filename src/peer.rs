use std::{
    fs::OpenOptions,
    io::Read,
    net::{SocketAddr, SocketAddrV6, TcpStream, ToSocketAddrs},
    sync::{mpsc, Arc, Mutex},
    thread,
    vec::IntoIter,
};

use bitcoin_hashes::{hex::FromHex, sha256d};
use bitcoin_hashes::{sha256, Hash};

use crate::{
    error::CustomError,
    message::{Message, MessageHeader},
    messages::{
        get_headers::GetHeaders,
        headers::{BlockHeader, Headers},
        ver_ack::VerAck,
        version::Version,
    },
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
    NewHeaders(Headers),
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
                    // ver archivo y serialize los headers
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(true)
                        .append(true)
                        .open("store/headers.txt")
                        .unwrap();

                    let mut saved_headers_buffer = vec![];
                    file.read_to_end(&mut saved_headers_buffer).unwrap();

                    // WIP: START FROM LAST SAVED
                    let genesis = vec![Vec::from_hex(
                        "6FE28C0AB6F1B372C1A6A246AE63F74F931E8365E15A089C68D6190000000000",
                    )
                    .unwrap()];

                    let saved_headers = Headers::parse_headers(saved_headers_buffer.clone(), 0);
                    // let last_header = match saved_headers.last() {
                    //     Some(header) => Has(header),
                    //     None => genesis
                    // }

                    let last_header = saved_headers_buffer
                        .get(saved_headers_buffer.len() - 81..saved_headers_buffer.len() - 1);

                    let block_header_hashes = match last_header {
                        Some(header) => {
                            // genesis.append(&mut vec![sha256d::Hash::hash(header)
                            //     .to_byte_array()
                            //     .to_vec()]);

                            [sha256d::Hash::hash(header).to_byte_array().to_vec()].to_vec()

                            // genesis.append(&mut [val.to_vec()].to_vec());
                            // genesis.reverse();
                            // genesis
                        }
                        None => genesis,
                    };

                    println!("Tengo ahora mismo: {}", saved_headers.len());
                    println!("primero header: {:?}", saved_headers.get(0));
                    println!("segundo header: {:?}", saved_headers.get(1));
                    println!("tercero header: {:?}", saved_headers.get(2));
                    println!("Ultimo header: {:?}", saved_headers.last());

                    // println!("Genesis: {:?}", genesis);
                    println!("Block header hashes: {:?}", block_header_hashes);

                    // END WIP: START FROM LAST SAVED

                    // empezar un loop pidiendo headers nuevos desde el ultimo que tengo

                    let request = GetHeaders::new(version, block_header_hashes, vec![0; 32])
                        .send(&mut thread_stream);

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
                let response = match Headers::read(&mut thread_stream, response_header.payload_size)
                {
                    Ok(response) => response,
                    Err(error) => {
                        println!("Error al leer el mensaje: {}", error);
                        return;
                    }
                };

                peer_response_sender_clone
                    .send(PeerResponse::NewHeaders(response))
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
