use std::{
    net::{Ipv6Addr, SocketAddr, SocketAddrV6},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    vec::IntoIter,
};

use crate::{
    config::Config,
    logger::Logger,
    peer_worker::{PeerAction, PeerResponse, PeerWorker},
};

pub struct Node {
    pub address: SocketAddrV6,
    pub services: u64,
    pub version: i32,
    logger_sender: Sender<String>,
    peers_sender: mpsc::Sender<PeerAction>,
    peers_receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    peers_response_sender: Sender<PeerResponse>,
    peers_response_receiver: Receiver<PeerResponse>,
    peers: Vec<PeerWorker>,
}

impl Node {
    pub fn new(config: &Config, logger: &Logger) -> Self {
        let logger_sender = logger.get_sender();

        let (peers_sender, receiver) = mpsc::channel();
        let peers_receiver = Arc::new(Mutex::new(receiver));
        let (peers_response_sender, peers_response_receiver) = mpsc::channel();

        Self {
            address: SocketAddrV6::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0), config.port, 0, 0),
            services: 0x00,
            version: config.protocol_version,
            logger_sender,
            peers_sender,
            peers_receiver,
            peers_response_sender,
            peers_response_receiver,
            peers: vec![],
        }
    }

    pub fn connect(&mut self, addresses: IntoIter<SocketAddr>) {
        self.logger_sender
            .send(format!("Handshaking with {} nodes", addresses.len()))
            .unwrap();

        self.start_workers(addresses);

        self.execute(PeerAction::GetHeaders);
        let message = self.peers_response_receiver.recv().unwrap();

        if let PeerResponse::NewHeaders(headers) = message {
            self.logger_sender
                .send(format!("New headers: {}", headers))
                .unwrap();

            for i in 0..20 {
                self.execute(PeerAction::GetBlock(i.to_string()));
            }
        }

        // verificar que tengas todos los headers
        while let Ok(message) = self.peers_response_receiver.recv() {
            self.handle_peer_response(message);
        }
    }

    fn start_workers(&mut self, addresses: IntoIter<SocketAddr>) {
        let mut num_of_threads = 10;
        for address in addresses {
            if num_of_threads == 0 {
                break;
            }

            let peer = PeerWorker::new(
                address,
                self.address,
                self.services,
                self.version,
                self.peers_receiver.clone(),
                self.logger_sender.clone(),
                self.peers_response_sender.clone(),
            );

            self.peers.push(peer);

            num_of_threads -= 1;
        }
    }

    fn handle_peer_response(&mut self, message: PeerResponse) {
        // esto tendria que ser una funcion receptora de mensajes (mientras descargas headers podria llegarte una transaccion o un bloque o lo que sea)
        match message {
            PeerResponse::Block((block_hash, block)) => {
                self.logger_sender
                    .send(format!("Block {}: {}", block_hash, block))
                    .unwrap();
            }
            _ => {}
        }
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
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
