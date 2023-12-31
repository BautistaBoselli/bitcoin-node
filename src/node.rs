use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    vec::IntoIter,
};

use gtk::glib;

use crate::{
    config::Config,
    error::CustomError,
    gui::init::GUIEvents,
    logger::{send_log, Log, Logger},
    loops::{
        node_action_loop::{NodeAction, NodeActionLoop},
        peer_action_loop::PeerAction,
        pending_blocks_loop::pending_blocks_loop,
        tcp_listener_loop::TcpListenerLoop,
    },
    node_state::NodeState,
    peer::{request_headers, Peer},
};

/// Node es la estructura que representa nuestro nodo.
/// Los elementos son:
/// - address: Direccion del nodo.
/// - services: Servicios que ofrece el nodo.
/// - version: Version del nodo.
/// - client_only: Indica si el nodo es completo o solo cliente.
/// - logger_sender: Sender para enviar logs al logger.
/// - peer_action_sender: Sender para enviar acciones al los peers.
/// - peer_action_receiver: Receiver para recibir acciones del peer.
/// - node_action_sender: Sender para enviar acciones al nodo.
/// - node_action_receiver: Receiver para recibir acciones del nodo.
/// - tcp_listener_thread: Thread del loop para atender conexiones entrantes a este nodo.
/// - node_state_ref: Referencia al estado del nodo.
/// - npeers: Cantidad de peers.
pub struct Node {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    client_only: bool,
    logger_sender: mpsc::Sender<Log>,
    peer_action_sender: mpsc::Sender<PeerAction>,
    peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    pub node_action_sender: mpsc::Sender<NodeAction>,
    node_action_receiver: Option<mpsc::Receiver<NodeAction>>,
    tcp_listener_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
    node_state_ref: Arc<Mutex<NodeState>>,
    npeers: u8,
}

impl Node {
    /// Inicializa el nodo.
    /// Crea los channels necesarios para la comunicacion con los peers y el logger.
    pub fn new(
        config: &Config,
        logger: &Logger,
        node_state_ref: Arc<Mutex<NodeState>>,
    ) -> Result<Self, CustomError> {
        let logger_sender = logger.get_sender();
        let (peer_action_sender, receiver) = mpsc::channel();
        let peer_action_receiver = Arc::new(Mutex::new(receiver));
        let (node_action_sender, node_action_receiver) = mpsc::channel();

        let node = Self {
            address: SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), config.port, 0, 0),
            services: 0x00,
            version: config.protocol_version,
            client_only: config.client_only,
            logger_sender,
            peer_action_sender,
            peer_action_receiver,
            node_action_sender,
            node_action_receiver: Some(node_action_receiver),
            tcp_listener_thread: None,
            npeers: config.npeers,
            node_state_ref,
        };

        Ok(node)
    }

    /// Inicializa el nodo en un thread.
    /// Comienza el thread de pending_blocks_loop.
    /// Comienza el thread de tcp_listener_loop.
    /// Comienza la descarga de headers.
    /// Comienza el thread de node_action_loop.
    pub fn spawn(
        mut self,
        addresses: IntoIter<SocketAddr>,
        gui_sender: glib::Sender<GUIEvents>,
    ) -> JoinHandle<Result<(), CustomError>> {
        self.initialize_pending_blocks_loop();
        self.initialize_tcp_listener_loop();

        thread::spawn(move || -> Result<(), CustomError> {
            if let Err(error) = self.connect(addresses, self.npeers) {
                send_log(&self.logger_sender, Log::Error(error));
            }
            if let Err(error) = self.initialize_ibd() {
                send_log(&self.logger_sender, Log::Error(error));
            }
            if let Err(error) = self.initialize_event_loop(gui_sender) {
                send_log(&self.logger_sender, Log::Error(error));
            }
            Ok(())
        })
    }

    fn connect(
        &mut self,
        addresses: IntoIter<SocketAddr>,
        mut number_of_peers: u8,
    ) -> Result<(), CustomError> {
        send_log(
            &self.logger_sender,
            Log::Message(format!(
                "Handshaking with {} nodes ({} available)",
                number_of_peers,
                addresses.len()
            )),
        );

        let mut peers = vec![];

        for address in addresses {
            if number_of_peers == 0 {
                break;
            }

            match Peer::call(
                address,
                self.address,
                self.services,
                self.version,
                self.peer_action_receiver.clone(),
                self.logger_sender.clone(),
                self.node_action_sender.clone(),
            ) {
                Ok(peer) => {
                    peers.push(peer);
                    number_of_peers -= 1;
                }
                Err(error) => {
                    send_log(
                        &self.logger_sender,
                        Log::Message(format!("Error connecting to peer: {:?}", error)),
                    );
                }
            };
        }

        let mut node_state = self.node_state_ref.lock()?;
        node_state.append_peers(peers);
        Ok(())
    }

    fn initialize_pending_blocks_loop(&self) {
        pending_blocks_loop(
            self.node_state_ref.clone(),
            self.peer_action_sender.clone(),
            self.logger_sender.clone(),
        );
    }

    fn initialize_tcp_listener_loop(&mut self) {
        if !self.client_only {
            self.tcp_listener_thread = Some(TcpListenerLoop::spawn(
                self.logger_sender.clone(),
                self.node_state_ref.clone(),
                self.address,
                self.services,
                self.version,
                self.peer_action_receiver.clone(),
                self.node_action_sender.clone(),
            ));
        }
    }

    fn initialize_ibd(&self) -> Result<(), CustomError> {
        let mut node_state = self.node_state_ref.lock()?;
        let last_header = node_state.get_last_header_hash();
        let fastest_peer = node_state.get_fastest_peer();

        if let Some(fastest_peer) = fastest_peer {
            request_headers(
                last_header,
                self.version,
                &mut fastest_peer.stream,
                &self.logger_sender,
                &self.node_action_sender,
            )?;

            send_log(
                &self.logger_sender,
                Log::Message(format!(
                    "Starting headers download with fastest peer: {}",
                    fastest_peer.address.ip()
                )),
            );

            return Ok(());
        }

        drop(node_state);
        self.peer_action_sender
            .send(PeerAction::GetHeaders(last_header))?;
        Ok(())
    }

    fn initialize_event_loop(
        &mut self,
        gui_sender: glib::Sender<GUIEvents>,
    ) -> Result<(), CustomError> {
        if let Some(receiver) = self.node_action_receiver.take() {
            NodeActionLoop::start(
                gui_sender,
                receiver,
                self.peer_action_sender.clone(),
                self.logger_sender.clone(),
                self.node_state_ref.clone(),
            );
            return Ok(());
        }
        Err(CustomError::CannotStartEventLoop)
    }
}

impl Drop for Node {
    /// Espera a que los threads de PeerActions terminen.
    /// Estos junto al NodeActionLoop son los únicos threads que modifican el estado del nodo, por lo que hace falta esperarlos.
    fn drop(&mut self) {
        if let Ok(mut node_state) = self.node_state_ref.lock() {
            let peers = node_state.get_peers();
            for _ in peers.iter() {
                self.peer_action_sender.send(PeerAction::Terminate).unwrap();
            }
            for peer in peers {
                if let Some(thread) = peer.peer_action_thread.take() {
                    if let Err(error) = thread.join() {
                        println!("Error joining thread: {:?}", error);
                    }
                }
            }
        }
    }
}
