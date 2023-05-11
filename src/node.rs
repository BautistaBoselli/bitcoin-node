use std::{
    fs::OpenOptions,
    io::{Read, Write},
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{mpsc, Arc, Mutex},
    thread,
    vec::IntoIter,
};

use crate::{
    config::Config,
    error::CustomError,
    logger::Logger,
    messages::{
        headers::Headers,
        inv::{Inventory, InventoryType},
    },
    peer::{Peer, PeerAction, PeerResponse},
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

        let peers_sender_clone = peers_sender.clone();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open("store/headers.txt")?;

        let mut saved_headers_buffer = vec![];
        file.read_to_end(&mut saved_headers_buffer)?;

        let mut headers = match Headers::parse_headers(saved_headers_buffer) {
            Ok(headers) => headers,
            Err(_) => vec![],
        };

        let last_header = headers.last().map(|header| header.hash());

        peers_sender_clone.send(PeerAction::GetHeaders(last_header))?;

        // thread que escucha los mensajes de los peers
        let thread = thread::spawn(move || -> Result<(), CustomError> {
            loop {
                while let Ok(message) = peers_response_receiver.recv() {
                    match message {
                        PeerResponse::Block((block_hash, block)) => {
                            let mut filename = String::with_capacity(2 * block_hash.len());
                            for byte in block_hash {
                                filename.push_str(format!("{:02X}", byte).as_str());
                            }

                            let mut file = OpenOptions::new()
                                .read(true)
                                .write(true)
                                .create(true)
                                .open(format!("store/blocks/{}.txt", filename))?;

                            file.write_all(&block)?;
                        }
                        PeerResponse::NewHeaders(mut new_headers) => {
                            let mut file = OpenOptions::new()
                                .read(true)
                                .write(true)
                                .create(true)
                                .append(true)
                                .open("store/headers.txt")?;

                            file.write_all(&new_headers.serialize_headers())?;

                            let new_headers_count = new_headers.headers.len();
                            new_headers
                                .headers
                                .iter()
                                .filter(|header| header.timestamp > 1683514800)
                                .collect::<Vec<_>>()
                                .chunks(5)
                                .for_each(|headers| {
                                    let inventory = headers
                                        .iter()
                                        .map(|header| {
                                            Inventory::new(InventoryType::GetBlock, header.hash())
                                        })
                                        .collect();
                                    peers_sender_clone
                                        .send(PeerAction::GetData(inventory))
                                        .unwrap();
                                });
                            headers.append(&mut new_headers.headers);
                            println!(
                                "Hay {} headers (nuevos {})",
                                headers.len(),
                                new_headers_count
                            );
                        }
                        PeerResponse::GetHeadersError => {
                            let last_header = headers.last().map(|header| header.hash());
                            peers_sender_clone.send(PeerAction::GetHeaders(last_header))?;
                        }

                        _ => {}
                    }
                }
            }
        });
        Ok(Self {
            address: SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), config.port, 0, 0),
            services: 0x00,
            version: config.protocol_version,
            logger_sender,
            peers_sender,
            peers_receiver,
            peers_response_sender,
            peers: vec![],
            event_loop_thread: Some(thread),
        })
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
                thread.join().unwrap();
            }
            if let Some(thread) = worker.stream_listener_thread.take() {
                thread.join().unwrap();
            }
        }
        self.event_loop_thread.take().unwrap().join().unwrap();
    }
}
