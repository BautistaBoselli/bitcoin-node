use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    message::Message,
    messages::{get_data::GetData, inv::Inventory},
    peer::{request_headers, PeerAction, PeerResponse},
};

pub struct PeerActionThread {
    pub receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
    pub version: i32,
    pub stream: TcpStream,
    pub logger_sender: mpsc::Sender<String>,
    pub peer_response_sender: mpsc::Sender<PeerResponse>,
}

impl PeerActionThread {
    pub fn spawn(
        receiver: Arc<Mutex<mpsc::Receiver<PeerAction>>>,
        version: i32,
        stream: TcpStream,
        logger_sender: mpsc::Sender<String>,
        peer_response_sender: mpsc::Sender<PeerResponse>,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut peer_action_thread = Self {
                receiver,
                version,
                stream,
                logger_sender,
                peer_response_sender,
            };
            peer_action_thread.event_loop()
        })
    }

    pub fn event_loop(&mut self) -> Result<(), CustomError> {
        loop {
            let peer_message = self
                .receiver
                .lock()
                .map_err(|_| CustomError::CannotLockGuard)?
                .recv()?;
            match peer_message {
                PeerAction::GetHeaders(last_header) => self.handle_getheaders(last_header)?,
                PeerAction::GetData(inventories) => self.handle_getdata(inventories)?,
                PeerAction::Terminate => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_getdata(&mut self, inventories: Vec<Inventory>) -> Result<(), CustomError> {
        println!("Enviando getdata...");
        let inventories_clone = inventories.clone();
        let request = GetData::new(inventories).send(&mut self.stream);
        if request.is_err() {
            self.logger_sender.send("Error pidiendo data".to_string())?;
            self.peer_response_sender
                .send(PeerResponse::GetDataError(inventories_clone))?;
        };
        Ok(())
    }

    fn handle_getheaders(&mut self, last_header: Option<Vec<u8>>) -> Result<(), CustomError> {
        request_headers(
            last_header,
            self.version,
            &mut self.stream,
            &self.logger_sender,
            &self.peer_response_sender,
        )
    }
}
