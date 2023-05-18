use std::{
    fs::OpenOptions,
    io::Read,
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{mpsc, Arc, Mutex},
    thread,
    vec::IntoIter,
};

use crate::{
    config::Config,
    error::CustomError,
    logger::Logger,
    messages::headers::Headers,
    peer::{Peer, PeerAction, PeerResponse},
    threads::node::NodeThread,
};
pub struct Node {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    logger_sender: mpsc::Sender<String>,
    peers_sender: mpsc::Sender<PeerAction>,
    peers_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    peers_response_sender: mpsc::Sender<PeerResponse>,
    peers: Vec<Peer>,
    pub event_loop_thread: Option<thread::JoinHandle<Result<(), CustomError>>>,
}

impl Node {
    pub fn new(config: &Config, logger: &Logger) -> Result<Self, CustomError> {
        let logger_sender = logger.get_sender();

        let (peers_sender, receiver) = mpsc::channel();
        let peers_receiver = Arc::new(Mutex::new(receiver));
        let (peers_response_sender, peers_response_receiver) = mpsc::channel();

        let mut headers_file = open_new_file(String::from("store/headers.bin"))?;

        let mut saved_headers_buffer = vec![];
        headers_file.read_to_end(&mut saved_headers_buffer)?;

        let headers = match Headers::parse_headers(saved_headers_buffer) {
            Ok(headers) => headers,
            Err(_) => vec![],
        };

        let last_header = headers.last().map(|header| header.hash());
        peers_sender.send(PeerAction::GetHeaders(last_header))?;

        let mut node = Self {
            address: SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), config.port, 0, 0),
            services: 0x00,
            version: config.protocol_version,
            logger_sender: logger_sender.clone(),
            peers_sender,
            peers_receiver,
            peers_response_sender,
            peers: vec![],
            event_loop_thread: None,
        };
        node.event_loop_thread = Some(NodeThread::spawn(
            peers_response_receiver,
            headers_file,
            node.peers_sender.clone(),
            headers,
            logger_sender.clone(),
        ));
        Ok(node)
    }

    pub fn connect(&mut self, addresses: IntoIter<SocketAddr>) -> Result<(), CustomError> {
        self.logger_sender
            .send(format!("Handshaking with {} nodes", addresses.len()))?;

        let mut num_of_threads = 10;
        for address in addresses {
            if num_of_threads == 0 {
                break;
            }

            let peer = Peer::new(
                address,
                self.address,
                self.services,
                self.version,
                self.peers_receiver.clone(),
                self.logger_sender.clone(),
                self.peers_response_sender.clone(),
            )?;
            self.peers.push(peer);

            num_of_threads -= 1;
        }
        Ok(())

        // verificar que tengas todos los headers
    }

    pub fn execute(&self, peer_message: PeerAction) -> Result<(), CustomError> {
        self.peers_sender.send(peer_message)?;
        Ok(())
    }
}

pub fn open_new_file(path_to_file: String) -> Result<std::fs::File, CustomError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .append(true)
        .open(path_to_file)?;
    Ok(file)
}

impl Drop for Node {
    fn drop(&mut self) {
        // for _ in &mut self.peers {
        //     self.peers_sender.send(PeerAction::Terminate).unwrap();
        // }

        self.logger_sender
            .send("Shutting down all workers.".to_string())
            .unwrap();

        for worker in &mut self.peers {
            if let Some(thread) = worker.node_listener_thread.take() {
                if let Err(error) = thread.join() {
                    println!("Error joining thread: {:?}", error);
                }
            }
            if let Some(thread) = worker.stream_listener_thread.take() {
                if let Err(error) = thread.join() {
                    println!("Error joining thread: {:?}", error);
                }
            }
        }
        self.event_loop_thread.take().map(|thread| {
            if let Err(error) = thread.join() {
                println!("Error joining event loop thread: {:?}", error);
            }
        });
    }
}
