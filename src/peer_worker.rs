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
    NewHeaders(String),
    GetHeadersError,
    Block((String, String)),
    GetBlockError(String),
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

            event_loop(receiver, peer, peers_response_sender);
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}

fn event_loop(
    receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    peer: Peer,
    peers_response_sender: mpsc::Sender<PeerResponse>,
) {
    loop {
        let peer_message = receiver.lock().unwrap().recv().unwrap();
        match peer_message {
            PeerAction::GetHeaders => handle_getheaders(&peer, &peers_response_sender),
            PeerAction::GetBlock(block_header) => {
                handle_getblock(&peer, block_header, &peers_response_sender)
            }
            PeerAction::Terminate => {
                break;
            }
        }
    }
}

fn handle_getheaders(peer: &Peer, peers_response_sender: &mpsc::Sender<PeerResponse>) {
    thread::sleep(Duration::from_millis(2000));

    let headers = match peer.get_headers() {
        Ok(headers) => headers,
        Err(_) => {
            peers_response_sender
                .send(PeerResponse::GetHeadersError)
                .unwrap();
            return;
        }
    };

    peers_response_sender
        .send(PeerResponse::NewHeaders(headers))
        .unwrap();
}

fn handle_getblock(
    peer: &Peer,
    block_header: String,
    peers_response_sender: &mpsc::Sender<PeerResponse>,
) {
    thread::sleep(Duration::from_millis(1000));

    let block = match peer.get_block(&block_header) {
        Ok(block) => block,
        Err(_) => {
            peers_response_sender
                .send(PeerResponse::GetBlockError(block_header))
                .unwrap();
            return;
        }
    };

    peers_response_sender
        .send(PeerResponse::Block((block_header, block)))
        .unwrap();
}
