use std::{
    net::{SocketAddrV6, TcpListener},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    node_state::NodeState,
};

pub struct TcpListenerLoop {
    logger_sender: mpsc::Sender<Log>,
    node_state_ref: Arc<Mutex<NodeState>>,
    address: SocketAddrV6,
}

impl TcpListenerLoop {
    #[must_use]
    /// Inicializa el loop de eventos en un thread.
    pub fn spawn(
        logger_sender: mpsc::Sender<Log>,
        node_state_ref: Arc<Mutex<NodeState>>,
        address: SocketAddrV6,
    ) -> JoinHandle<Result<(), CustomError>> {
        thread::spawn(move || -> Result<(), CustomError> {
            let mut thread = Self {
                logger_sender,
                node_state_ref,
                address,
            };
            thread.event_loop()
        })
    }

    fn event_loop(&mut self) -> Result<(), CustomError> {
        let listener = TcpListener::bind(self.address)?;

        // CHEQUEAR: solo iniciar cuando el nodo esta sincronizado con la red
        send_log(
            &self.logger_sender,
            Log::Message(String::from("Server started...")),
        );

        for stream in listener.incoming() {
            let stream = stream?;
            send_log(
                &self.logger_sender,
                Log::Message(format!("New connection: {:?}", stream.peer_addr())),
            );

            // let a = MessageHeader::read(&mut stream)?;
            // let version = Version::read(&mut stream, a.payload_size)?;
            // println!("version: {:?}", version);

            // version.send(&mut stream)?;
            // println!("version enviado");

            // let verack = VerAck::new();
            // verack.send(&mut stream)?;
            // println!("verack enviado");

            // let a = MessageHeader::read(&mut stream)?;
            // let verack = VerAck::read(&mut stream, a.payload_size)?;
            // println!("verack recibido: {:?}", verack);
        }

        println!("Terminado");

        Ok(())
    }
}
