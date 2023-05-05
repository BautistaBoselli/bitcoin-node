use std::{
    net::{SocketAddr, SocketAddrV6},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use crate::peer::Peer;

pub enum PeerAction {
    GetHeaders,
    GetBlock(String),
    Terminate,
}

pub enum PeerResponse {
    HandshakeSuccess,
    NewHeaders(String),
}

pub struct PeerWorker {
    pub id: SocketAddr,
    pub thread: Option<thread::JoinHandle<()>>,
}

impl PeerWorker {
    pub fn new(
        id: SocketAddr,
        sender_address: SocketAddrV6,
        services: u64,
        version: i32,
        receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        logger_sender: mpsc::Sender<String>,
        peers_response_sender: mpsc::Sender<PeerResponse>,
    ) -> Self {
        let thread = thread::spawn(move || {
            let mut peer = match Peer::new(id, services, version, logger_sender.clone()) {
                Ok(peer) => peer,
                Err(error) => {
                    // deberia retornarse el error de handshake al main
                    logger_sender
                        .send(format!("ERROR: {} {}", error, id.ip()))
                        .unwrap();
                    return;
                }
            };
            peer.handshake(sender_address).unwrap();

            event_loop(receiver, logger_sender, peer, peers_response_sender);
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}

fn event_loop(
    receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    logger_sender: mpsc::Sender<String>,
    peer: Peer,
    peers_response_sender: mpsc::Sender<PeerResponse>,
) {
    loop {
        let peer_message = receiver.lock().unwrap().recv().unwrap();
        match peer_message {
            PeerAction::GetHeaders => {
                handle_getheaders(&logger_sender, &peer, &peers_response_sender)
            }
            PeerAction::GetBlock(job) => {
                handle_getblock(&logger_sender, &peer, job);
            }
            PeerAction::Terminate => {
                break;
            }
        }
    }
}

fn handle_getheaders(
    logger_sender: &mpsc::Sender<String>,
    peer: &Peer,
    peers_response_sender: &mpsc::Sender<PeerResponse>,
) {
    logger_sender
        .send(format!("peer {}: getting headers...", peer.address.ip()))
        .unwrap();
    thread::sleep(Duration::from_millis(2000));
    peers_response_sender
        .send(PeerResponse::NewHeaders(
            "<<< NUEVOS HEADERS >>>".to_string(),
        ))
        .unwrap();
}

fn handle_getblock(logger_sender: &mpsc::Sender<String>, peer: &Peer, job: String) {
    thread::sleep(Duration::from_millis(1000));
    logger_sender
        .send(format!(
            "peer {}: getting block {}...",
            peer.address.ip(),
            job
        ))
        .unwrap();
}
