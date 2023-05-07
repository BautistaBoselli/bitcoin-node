use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    vec::IntoIter,
};

use chrono::{TimeZone, Utc};

use crate::{
    config::Config,
    logger::Logger,
    peer::{Peer, PeerAction, PeerResponse},
};

pub struct Node {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    logger_sender: Sender<String>,
    peers_sender: mpsc::Sender<PeerAction>,
    peers_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    peers_response_sender: Sender<PeerResponse>,
    peers: Vec<Peer>,
    pub event_loop_thread: Option<thread::JoinHandle<()>>,
}

impl Node {
    pub fn new(config: &Config, logger: &Logger) -> Self {
        let logger_sender = logger.get_sender();

        let (peers_sender, receiver) = mpsc::channel();
        let peers_receiver = Arc::new(Mutex::new(receiver));
        let (peers_response_sender, peers_response_receiver) = mpsc::channel();

        let logger_sender_clone = logger_sender.clone();
        let peers_sender_clone = peers_sender.clone();
        let thread = thread::spawn(move || loop {
            while let Ok(message) = peers_response_receiver.recv() {
                match message {
                    PeerResponse::Block((block_hash, block)) => {
                        logger_sender_clone
                            .send(format!("Block {}: {}", block_hash, block))
                            .unwrap();
                    }
                    PeerResponse::NewHeaders(new_headers) => {
                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .append(true)
                            .open("store/headers.txt")
                            .unwrap();

                        file.write_all(&new_headers.serialize_headers()).unwrap();

                        logger_sender_clone
                            .send(format!("New headers: {:?}", new_headers.headers.last()))
                            .unwrap();

                        println!("nuevos primero header: {:?}", new_headers.headers.get(0));
                        println!("nuevos segundo header: {:?}", new_headers.headers.get(1));
                        println!("nuevos tercero header: {:?}", new_headers.headers.get(2));
                        println!("nuevos Ultimo header: {:?}", new_headers.headers.last());

                        // new_headers.headers.iter().for_each(|header| {
                        //     // if header.timestamp > Utc.with_ymd_and_hms(2014, 11, 28, 12, 0, 9) {}
                        //     peers_sender_clone
                        //         .send(PeerAction::GetBlock(header.to_string()))
                        //         .unwrap();
                        // });
                        for i in 0..20 {
                            peers_sender_clone
                                .send(PeerAction::GetBlock(i.to_string()))
                                .unwrap();
                        }
                    }
                    _ => {}
                }
            }
        });

        Self {
            address: SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), config.port, 0, 0),
            services: 0x00,
            version: config.protocol_version,
            logger_sender,
            peers_sender,
            peers_receiver,
            peers_response_sender,
            peers: vec![],
            event_loop_thread: Some(thread),
        }
    }

    pub fn connect(&mut self, addresses: IntoIter<SocketAddr>) {
        self.logger_sender
            .send(format!("Handshaking with {} nodes", addresses.len()))
            .unwrap();

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
            )
            .unwrap();

            self.peers.push(peer);

            num_of_threads -= 1;
        }

        // verificar que tengas todos los headers
    }

    pub fn execute(&self, peer_message: PeerAction) {
        self.peers_sender.send(peer_message).unwrap();
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        for _ in &mut self.peers {
            self.peers_sender.send(PeerAction::Terminate).unwrap();
        }

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
