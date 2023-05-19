use std::{
    net::{SocketAddr, SocketAddrV6, TcpStream},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    error::CustomError,
    message::{Message, MessageHeader},
    messages::{
        block::Block, get_headers::GetHeaders, headers::Headers, inv::Inventory,
        send_headers::SendHeaders, ver_ack::VerAck, version::Version,
    },
    network::{get_address_v6, open_stream},
    threads::{peer_action::PeerActionThread, peer_response::PeerResponseThread},
};

pub const GENESIS: [u8; 32] = [
    111, 226, 140, 10, 182, 241, 179, 114, 193, 166, 162, 70, 174, 99, 247, 79, 147, 30, 131, 101,
    225, 90, 8, 156, 104, 214, 25, 0, 0, 0, 0, 0,
];

pub enum PeerAction {
    GetHeaders(Option<Vec<u8>>),
    GetData(Vec<Inventory>),
    Terminate,
}

pub enum PeerResponse {
    NewHeaders(Headers),
    GetHeadersError,
    Block((Vec<u8>, Block)),
    GetDataError(Vec<Inventory>),
}

pub struct Peer {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    pub stream: TcpStream,
    pub node_listener_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
    pub stream_listener_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
}

impl Peer {
    pub fn new(
        address: SocketAddr,
        sender_address: SocketAddrV6,
        services: u64,
        version: i32,
        receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        mut logger_sender: mpsc::Sender<String>,
        peers_response_sender: mpsc::Sender<PeerResponse>,
    ) -> Result<Self, CustomError> {
        let address = get_address_v6(address);
        let stream = open_stream(address)?;

        let mut peer = Self {
            address,
            node_listener_thread: None,
            stream_listener_thread: None,
            services,
            version,
            stream,
        };

        peer.handshake(sender_address, &mut logger_sender)?;
        peer.spawn_threads(receiver, peers_response_sender, logger_sender)?;
        Ok(peer)
    }

    pub fn handshake(
        &mut self,
        sender_address: SocketAddrV6,
        logger_sender: &mut mpsc::Sender<String>,
    ) -> Result<(), CustomError> {
        self.share_versions(sender_address)?;
        self.share_veracks()?;
        SendHeaders::new().send(&mut self.stream)?;

        logger_sender.send(format!("Successful handshake with {}", self.address.ip()))?;

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

    fn spawn_threads(
        &mut self,
        receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        peer_response_sender: mpsc::Sender<PeerResponse>,
        logger_sender: mpsc::Sender<String>,
    ) -> Result<(), CustomError> {
        //thread que escucha al nodo
        self.node_listener_thread = Some(PeerActionThread::spawn(
            receiver,
            self.version,
            self.stream.try_clone()?,
            logger_sender.clone(),
            peer_response_sender.clone(),
        ));

        //Thread que escucha el stream
        self.stream_listener_thread = Some(PeerResponseThread::spawn(
            self.version,
            self.stream.try_clone()?,
            logger_sender,
            peer_response_sender,
        ));
        Ok(())
    }
}

pub fn request_headers(
    last_header: Option<Vec<u8>>,
    version: i32,
    stream: &mut TcpStream,
    logger_sender: &mpsc::Sender<String>,
    peer_response_sender: &mpsc::Sender<PeerResponse>,
) -> Result<(), CustomError> {
    let block_header_hashes = match last_header {
        Some(header) => [header].to_vec(),
        None => [GENESIS.to_vec()].to_vec(),
    };

    let request = GetHeaders::new(version, block_header_hashes, vec![0; 32]).send(stream);
    if request.is_err() {
        logger_sender.send("Error pidiendo headers".to_string())?;
        peer_response_sender.send(PeerResponse::GetHeadersError)?;
    }
    Ok(())
}
