use std::{
    net::{SocketAddrV6, TcpListener},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
    peer::Peer,
};

use super::{node_action_loop::NodeAction, peer_action_loop::PeerAction};

/// TcpListenerLoop es el loop de eventos que se encarga de escuchar conexiones entrantes.
/// Cada vez que se recibe una conexión, inicializa un nuevo Peer y contesta el handshake.
/// Luego, agrega el nuevo Peer a la lista de peers del nodo
/// Los elementos son:
/// - logger_sender: Sender para enviar logs al logger
/// - node_state_ref: Referencia al estado del nodo
/// - address: Dirección del nodo
/// - services: Servicios que ofrece el nodo
/// - version: Versión del protocolo que maneja el nodo
/// - peer_action_receiver: Receiver para recibir acciones de los peers
/// - node_action_sender: Sender para enviar acciones al nodo
pub struct TcpListenerLoop {
    logger_sender: mpsc::Sender<Log>,
    node_state_ref: Arc<Mutex<NodeState>>,
    address: SocketAddrV6,
    services: u64,
    version: i32,
    peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    node_action_sender: mpsc::Sender<NodeAction>,
}

impl TcpListenerLoop {
    #[must_use]
    /// Inicializa el loop de eventos en un thread.
    pub fn spawn(
        logger_sender: mpsc::Sender<Log>,
        node_state_ref: Arc<Mutex<NodeState>>,
        address: SocketAddrV6,
        services: u64,
        version: i32,
        peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        node_action_sender: mpsc::Sender<NodeAction>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut thread = Self {
                logger_sender,
                node_state_ref,
                address,
                services,
                version,
                peer_action_receiver,
                node_action_sender,
            };
            thread.event_loop()
        })
    }

    fn event_loop(&mut self) -> Result<(), CustomError> {
        let listener = TcpListener::bind(self.address)?;
        send_log(
            &self.logger_sender,
            Log::Message(String::from("Server started...")),
        );

        for stream in listener.incoming() {
            let stream = stream?;
            let peer_address = stream.peer_addr()?;
            send_log(
                &self.logger_sender,
                Log::Message(format!("New connection: {:?}", peer_address)),
            );

            let new_peer = Peer::answer(
                stream,
                self.address,
                self.services,
                self.version,
                self.peer_action_receiver.clone(),
                self.logger_sender.clone(),
                self.node_action_sender.clone(),
            )?;

            let mut node_state = self.node_state_ref.lock()?;
            node_state.append_peers(vec![new_peer]);
            drop(node_state);
        }

        Ok(())
    }
}
