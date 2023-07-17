use std::{
    collections::HashMap,
    net::{SocketAddr, SocketAddrV6, TcpStream},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    loops::{peer_action_loop::PeerActionLoop, peer_stream_loop::PeerStreamLoop},
    message::{Message, MessageHeader},
    messages::{
        block::Block, get_headers::GetHeaders, headers::Headers, send_headers::SendHeaders,
        transaction::Transaction, ver_ack::VerAck, version::Version,
    },
    structs::inventory::Inventory,
    utils::{get_address_v6, open_stream},
};

/// GENESIS es el hash del bloque genesis de la blockchain de Bitcoin.
pub const GENESIS: [u8; 32] = [
    111, 226, 140, 10, 182, 241, 179, 114, 193, 166, 162, 70, 174, 99, 247, 79, 147, 30, 131, 101,
    225, 90, 8, 156, 104, 214, 25, 0, 0, 0, 0, 0,
];

/// PeerAction es una enumeracion de las acciones que puede realizar un peer.
/// Las acciones son:
/// - GetHeaders: Solicita headers al peer.
/// - GetData: Solicita data al peer.
/// - SendTransaction: Envia una transaccion al peer.
pub enum PeerAction {
    GetHeaders(Option<Vec<u8>>),
    GetData(Vec<Inventory>),
    SendTransaction(Transaction),
}

/// NodeAction es una enumeracion de las acciones que puede realizar el nodo.
/// Las acciones son:
/// - NewHeaders: Recibe nuevos headers.
/// - GetHeadersError: Error al solicitar headers.
/// - Block: Recibe un bloque.
/// - GetDataError: Error al solicitar data.
/// - PendingTransaction: Recibe una transaccion.
/// - MakeTransaction: Solicitar una transaccion.
pub enum NodeAction {
    NewHeaders(Headers),
    GetHeadersError,
    Block((Vec<u8>, Block)),
    GetDataError(Vec<Inventory>),
    PendingTransaction(Transaction),
    MakeTransaction((HashMap<String, u64>, u64)),
    SendHeaders(SocketAddrV6),
}

/// Peer es una representacion de los Peers a los que nos conectamos, contiene los elementos necesarios para manejar la conexion con el peer.
/// Cada peer tiene dos threads asociados:
/// - peer_action_thread: Thread que escucha las acciones a realizar por el peer.
/// - peer_stream_thread: Thread que escucha el stream del peer.
///
/// Los elementos son:
/// - address: Direccion del peer.
/// - services: Servicios del peer.
/// - version: Version del peer.
/// - stream: Stream del peer.
/// - peer_action_thread: Thread que escucha las acciones a realizar por el peer.
/// - peer_stream_thread: Thread que escucha el stream del peer.
///
pub struct Peer {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    pub send_headers: bool,
    pub stream: TcpStream,
    pub peer_action_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
    pub peer_stream_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
}

impl Peer {
    /// Inicializa el peer.
    /// Realiza el handshake con el peer y crea los threads asociados.
    pub fn call(
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
            send_headers: false,
        };

        peer.call_handshake(sender_address, &mut logger_sender)?;
        peer.spawn_threads(peer_action_receiver, node_action_sender, logger_sender)?;
        Ok(peer)
    }

    pub fn answer(
        stream: TcpStream,
        sender_address: SocketAddrV6,
        services: u64,
        version: i32,
        peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        mut logger_sender: mpsc::Sender<Log>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> Result<Self, CustomError> {
        let mut peer = Self {
            address: get_address_v6(stream.peer_addr()?),
            peer_action_thread: None,
            peer_stream_thread: None,
            services,
            version,
            stream,
            send_headers: false,
        };

        println!("hola3");
        peer.answer_handshake(sender_address, &mut logger_sender)?;
        peer.spawn_threads(peer_action_receiver, node_action_sender, logger_sender)?;
        Ok(peer)
    }

    /// Realiza el handshake de Node con el Peer.
    fn call_handshake(
        &mut self,
        sender_address: SocketAddrV6,
        logger_sender: &mut mpsc::Sender<Log>,
    ) -> Result<(), CustomError> {
        Version::new(self.address, sender_address, self.version, self.services)
            .send(&mut self.stream)?;

        let response_header = MessageHeader::read(&mut self.stream)?;
        let version_response = Version::read(&mut self.stream, response_header.payload_size)
            .map_err(|_| CustomError::CannotHandshakeNode)?;
        self.version = version_response.version;
        self.services = version_response.services;

        let response_header = MessageHeader::read(&mut self.stream)?;
        VerAck::read(&mut self.stream, response_header.payload_size)
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        VerAck::new().send(&mut self.stream)?;
        SendHeaders::new().send(&mut self.stream)?;

        send_log(
            logger_sender,
            Log::Message(format!("Successful handshake with {}", self.address.ip())),
        );

        Ok(())
    }

    fn answer_handshake(
        &mut self,
        sender_address: SocketAddrV6,
        logger_sender: &mut mpsc::Sender<Log>,
    ) -> Result<(), CustomError> {
        println!("hola");
        let response_header = MessageHeader::read(&mut self.stream)?;
        let version_response = Version::read(&mut self.stream, response_header.payload_size)
            .map_err(|_| CustomError::CannotHandshakeNode)?;
        self.version = version_response.version;
        self.services = version_response.services;

        Version::new(self.address, sender_address, self.version, self.services)
            .send(&mut self.stream)?;
        VerAck::new().send(&mut self.stream)?;

        let response_header = MessageHeader::read(&mut self.stream)?;
        VerAck::read(&mut self.stream, response_header.payload_size)
            .map_err(|_| CustomError::CannotHandshakeNode)?;

        send_log(
            logger_sender,
            Log::Message(format!("Successful handshake with {}", self.address.ip())),
        );

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
            self.address,
            self.stream.try_clone()?,
            logger_sender,
            node_action_sender,
        ));
        Ok(())
    }

    pub fn send(&mut self, message: impl Message) -> Result<(), CustomError> {
        message.send(&mut self.stream)
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
