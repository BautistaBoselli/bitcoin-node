use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{mpsc, Arc, Mutex},
    thread,
    vec::IntoIter,
};

use gtk::glib;

use crate::{
    config::Config,
    error::CustomError,
    gui::init::GUIEvents,
    logger::{send_log, Log, Logger},
    loops::{node_action_loop::NodeActionLoop, pending_blocks_loop::pending_blocks_loop},
    node_state::NodeState,
    peer::{NodeAction, Peer, PeerAction},
};
pub struct Node {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    logger_sender: mpsc::Sender<Log>,
    peer_action_sender: mpsc::Sender<PeerAction>,
    peer_action_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    pub node_action_sender: mpsc::Sender<NodeAction>,
    node_action_receiver: Option<mpsc::Receiver<NodeAction>>,
    node_state_ref: Arc<Mutex<NodeState>>,
    peers: Vec<Peer>,
    npeers: u8,
    // pub event_loop_thread: Option<thread::JoinHandle<()>>,
}

impl Node {
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
            logger_sender,
            peer_action_sender,
            peer_action_receiver,
            node_action_sender,
            node_action_receiver: Some(node_action_receiver),
            peers: vec![],
            npeers: config.npeers,
            node_state_ref,
        };

        Ok(node)
    }

    pub fn spawn(mut self, addresses: IntoIter<SocketAddr>, gui_sender: glib::Sender<GUIEvents>) {
        self.initialize_pending_blocks_loop();

        thread::spawn(move || -> Result<(), CustomError> {
            self.connect(addresses, self.npeers);

            if let Err(error) = self.initialize_ibd() {
                send_log(&self.logger_sender, Log::Error(error));
            }
            if let Err(error) = self.initialize_event_loop(gui_sender.clone()) {
                send_log(&self.logger_sender, Log::Error(error));
            }
            Ok(())
        });
    }

    pub fn connect(&mut self, addresses: IntoIter<SocketAddr>, mut number_of_peers: u8) {
        send_log(
            &self.logger_sender,
            Log::Message(format!(
                "Handshaking with {} nodes ({} available)",
                number_of_peers,
                addresses.len()
            )),
        );

        for address in addresses {
            if number_of_peers == 0 {
                break;
            }

            match Peer::new(
                address,
                self.address,
                self.services,
                self.version,
                self.peer_action_receiver.clone(),
                self.logger_sender.clone(),
                self.node_action_sender.clone(),
            ) {
                Ok(peer) => {
                    self.peers.push(peer);
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
    }

    fn initialize_pending_blocks_loop(&self) {
        pending_blocks_loop(
            self.node_state_ref.clone(),
            self.peer_action_sender.clone(),
            self.logger_sender.clone(),
        );
    }

    fn initialize_ibd(&self) -> Result<(), CustomError> {
        let node_state = self
            .node_state_ref
            .lock()
            .map_err(|_| CustomError::CannotLockGuard)?;
        let last_header = node_state.get_last_header_hash();
        drop(node_state);
        self.peer_action_sender
            .send(PeerAction::GetHeaders(last_header))?;
        Ok(())
    }

    fn initialize_event_loop(
        &mut self,
        // node_action_receiver: mpsc::Receiver<NodeAction>,
        gui_sender: glib::Sender<GUIEvents>,
    ) -> Result<(), CustomError> {
        if let Some(receiver) = self.node_action_receiver.take() {
            NodeActionLoop::start(
                receiver,
                self.peer_action_sender.clone(),
                self.logger_sender.clone(),
                gui_sender,
                self.node_state_ref.clone(),
            );
            return Ok(());
        }
        Err(CustomError::CannotStartEventLoop)
    }

    pub fn execute(&self, peer_message: PeerAction) -> Result<(), CustomError> {
        self.peer_action_sender.send(peer_message)?;
        Ok(())
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        for worker in &mut self.peers {
            if let Some(thread) = worker.peer_action_thread.take() {
                if let Err(error) = thread.join() {
                    println!("Error joining thread: {:?}", error);
                }
            }
            if let Some(thread) = worker.peer_stream_thread.take() {
                if let Err(error) = thread.join() {
                    println!("Error joining thread: {:?}", error);
                }
            }
        }
    }
}
