use std::{
    collections::HashMap,
    net::{SocketAddr, SocketAddrV6, TcpStream},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    message::{Message, MessageHeader},
    messages::{
        block::Block, get_headers::GetHeaders, headers::Headers, inv::Inventory,
        send_headers::SendHeaders, transaction::Transaction, ver_ack::VerAck, version::Version,
    },
    network::{get_address_v6, open_stream},
    loops::{peer_action_loop::PeerActionLoop, peer_stream_loop::PeerStreamLoop},
};

pub const GENESIS: [u8; 32] = [
    111, 226, 140, 10, 182, 241, 179, 114, 193, 166, 162, 70, 174, 99, 247, 79, 147, 30, 131, 101,
    225, 90, 8, 156, 104, 214, 25, 0, 0, 0, 0, 0,
];

pub enum PeerAction {
    GetHeaders(Option<Vec<u8>>),
    GetData(Vec<Inventory>),
    SendTransaction(Transaction),
    Terminate,
}

pub enum NodeAction {
    NewHeaders(Headers),
    GetHeadersError,
    Block((Vec<u8>, Block)),
    GetDataError(Vec<Inventory>),
    PendingTransaction(Transaction),
    MakeTransaction((HashMap<String, u64>, u64)),
}

pub struct Peer {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    pub stream: TcpStream,
    pub peer_action_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
    pub peer_stream_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
}

impl Peer {
    pub fn new(
        address: SocketAddr,
        sender_address: SocketAddrV6,
        services: u64,
        version: i32,
        peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        mut logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> Result<Self, CustomError> {
        let stream = open_stream(address)?;
        let mut peer = Self {
            address: get_address_v6(address),
            peer_action_thread: None,
            peer_stream_thread: None,
            services,
            version,
            stream,
        };

        peer.handshake(sender_address, &mut logger_sender)?;
        peer.spawn_threads(peer_action_receiver, node_action_sender, logger_sender)?;
        Ok(peer)
    }

    pub fn handshake(
        &mut self,
        sender_address: SocketAddrV6,
        logger_sender: &mut mpsc::Sender<Log>,
    ) -> Result<(), CustomError> {
        self.share_versions(sender_address)?;
        self.share_veracks()?;
        SendHeaders::new().send(&mut self.stream)?;

        send_log(
            logger_sender,
            Log::Message(format!("Successful handshake with {}", self.address.ip())),
        );

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
        peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        node_action_sender: mpsc::Sender<NodeAction>,
        logger_sender: mpsc::Sender<Log>,
    ) -> Result<(), CustomError> {
        //thread que escucha al nodo
        self.peer_action_thread = Some(PeerActionLoop::spawn(
            peer_action_receiver,
            self.version,
            self.stream.try_clone()?,
            logger_sender.clone(),
            node_action_sender.clone(),
        ));

        //Thread que escucha el stream
        self.peer_stream_thread = Some(PeerStreamLoop::spawn(
            self.version,
            self.stream.try_clone()?,
            logger_sender,
            node_action_sender,
        ));
        Ok(())
    }
}

pub fn request_headers(
    last_header: Option<Vec<u8>>,
    version: i32,
    stream: &mut TcpStream,
    logger_sender: &mpsc::Sender<Log>,
    node_action_sender: &mpsc::Sender<NodeAction>,
) -> Result<(), CustomError> {
    let block_header_hashes = match last_header {
        Some(header) => [header].to_vec(),
        None => [GENESIS.to_vec()].to_vec(),
    };

    let request = GetHeaders::new(version, block_header_hashes, vec![0; 32]).send(stream);
    if request.is_err() {
        send_log(
            logger_sender,
            Log::Message("Error requesting headers".to_string()),
        );
        node_action_sender.send(NodeAction::GetHeadersError)?;
    }
    Ok(())
}
