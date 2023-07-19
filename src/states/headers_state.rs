use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::{get_headers::GetHeaders, headers::Headers},
    parser::BufferParser,
    peer::GENESIS,
    structs::block_header::BlockHeader,
    utils::open_new_file,
};

/// HeadersState es una estructura que contiene los elementos necesarios para manejar los headers.
/// Los elementos son:
/// - headers: Headers del nodo.
/// - logger_sender: Sender para enviar logs al logger.
/// - path: Path del archivo donde se guardan los headers.
/// - sync: Indica si los headers del nodo estan sincronizados con la red.
pub struct HeadersState {
    headers: Vec<BlockHeader>,
    logger_sender: Sender<Log>,
    path: String,
    sync: bool,
}

impl HeadersState {
    /// Inicializa los headers del nodo.
    /// Si el archivo donde se guardan los headers no existe, se crea.
    /// Si el archivo existe, se restauran los headers.
    pub fn new(path: String, logger_sender: Sender<Log>) -> Result<Self, CustomError> {
        let mut headers = Self {
            headers: Vec::new(),
            logger_sender,
            path,
            sync: false,
        };

        headers.restore()?;
        Ok(headers)
    }

    fn restore(&mut self) -> Result<(), CustomError> {
        let mut buffer = vec![];
        let mut file = open_new_file(self.path.clone(), true)?;
        file.read_to_end(&mut buffer)?;

        let mut parser = BufferParser::new(buffer);
        if parser.len() % 112 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        while !parser.is_empty() {
            let header = BlockHeader::parse_with_hash(parser.extract_buffer(112)?.to_vec())?;
            self.headers.push(header);
        }
        Ok(())
    }

    /// Devuelve todos los headers del nodo.
    pub fn get_all(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    /// Devuelve el hash del ultimo header del nodo.
    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.last().map(|header| header.hash())
    }

    /// Devuelve los ultimos count headers del nodo.
    pub fn get_last_headers(&self, count: usize) -> Vec<BlockHeader> {
        self.headers.iter().rev().take(count).cloned().collect()
    }

    /// Agrega los headers al nodo y los almacena.
    /// Tambien verifica si con los nuevos queda sincronizado con la red
    pub fn append_headers(&mut self, headers: &mut Headers) -> Result<(), CustomError> {
        let mut file = open_new_file(self.path.clone(), true)?;

        let mut buffer = vec![];
        for header in &headers.headers {
            let header_buffer: Vec<u8> = header.serialize_with_hash();
            buffer.extend(header_buffer);
        }

        file.write_all(buffer.as_slice())?;

        let headers_count = headers.headers.len();
        self.headers.append(&mut headers.headers);

        send_log(
            &self.logger_sender,
            Log::Message(format!(
                "There are {} headers, new {}",
                self.headers.len(),
                headers_count
            )),
        );

        self.verify_headers_sync(headers_count)?;
        Ok(())
    }

    /// Verifica si con los nuevos headers queda sincronizado con la red
    pub fn verify_headers_sync(&mut self, new_headers_count: usize) -> Result<(), CustomError> {
        if self.sync {
            return Ok(());
        }

        self.sync = new_headers_count < 2000;
        if self.sync {
            send_log(
                &self.logger_sender,
                Log::Message("headers sync completed".to_string()),
            );
        }
        Ok(())
    }

    /// Devuelve si los headers del nodo estan sincronizados con la red.
    pub fn is_synced(&self) -> bool {
        self.sync
    }

    pub fn get_headers(&self, get_headers: GetHeaders) -> Vec<BlockHeader> {
        let mut headers = vec![];
        let mut found = false;
        let peer_last_header = get_headers
            .block_locator_hashes
            .last()
            .unwrap_or(&GENESIS.to_vec())
            .clone();
        if let Some(last_header) = self.headers.last() {
            if peer_last_header == last_header.hash() {
                return vec![];
            }
        }

        if peer_last_header == GENESIS.to_vec() {
            found = true;
        }
        for header in &self.headers {
            if header.prev_block_hash == peer_last_header {
                found = true;
            }
            if found {
                headers.push(header.clone());
            }
            if headers.len() == 2000 || header.hash() == get_headers.hash_stop {
                break;
            }
        }

        if !found {
            self.headers
                .iter()
                .take(2000)
                .for_each(|header| headers.push(header.clone()));
        }
        headers
    }
}

#[cfg(test)]
mod tests {

    use std::{
        fs::{self, remove_file},
        sync::mpsc,
    };

    use super::*;

    #[test]
    fn headers_creation_empty() {
        let (logger_sender, _) = mpsc::channel();
        let headers =
            HeadersState::new("tests/non_existing_headers.bin".to_string(), logger_sender).unwrap();
        assert_eq!(headers.headers.len(), 0);

        remove_file("tests/non_existing_headers.bin").unwrap();
    }

    #[test]
    fn headers_creation_with_restore() {
        let (mut logger_sender, _) = mpsc::channel();
        let headers = HeadersState::new(
            "tests/test_headers.bin".to_string(),
            Sender::clone(&mut logger_sender),
        )
        .unwrap();
        assert_eq!(headers.headers.len(), 2);
    }

    #[test]
    fn headers_creation_with_restore_error() {
        let (mut logger_sender, _) = mpsc::channel();
        let headers = HeadersState::new(
            "tests/test_headers_error.bin".to_string(),
            Sender::clone(&mut logger_sender),
        );
        assert_eq!(headers.is_err(), true);
    }

    #[test]
    fn headers_get_all() {
        let (logger_sender, _) = mpsc::channel();
        let headers =
            HeadersState::new("tests/test_headers.bin".to_string(), logger_sender).unwrap();

        assert_eq!(headers.get_all().len(), 2);
    }

    #[test]
    fn headers_get_last_header_hash() {
        let (logger_sender, _) = mpsc::channel();
        let headers =
            HeadersState::new("tests/test_headers.bin".to_string(), logger_sender).unwrap();

        assert_eq!(
            headers.get_last_header_hash().unwrap(),
            vec![
                32, 120, 42, 0, 82, 85, 182, 87, 105, 110, 160, 87, 213, 185, 143, 52, 222, 252,
                247, 81, 150, 246, 79, 110, 234, 200, 2, 108, 0, 0, 0, 0
            ]
        );
    }

    #[test]
    fn headers_append_headers() {
        let (logger_sender, _) = mpsc::channel();
        fs::copy("tests/test_headers.bin", "tests/test_headers_append.bin").unwrap();
        let mut headers =
            HeadersState::new("tests/test_headers_append.bin".to_string(), logger_sender).unwrap();

        let mut new_headers = Headers::new();
        new_headers.headers.push(BlockHeader {
            prev_block_hash: vec![],
            merkle_root: vec![],
            version: 0,
            timestamp: 0,
            bits: 0,
            nonce: 0,
            hash: vec![],
        });

        headers.append_headers(&mut new_headers).unwrap();
        assert_eq!(headers.headers.len(), 3);

        remove_file("tests/test_headers_append.bin").unwrap();
    }

    #[test]

    fn headers_verify_headers_sync() {
        let (logger_sender, _) = mpsc::channel();
        let mut headers =
            HeadersState::new("tests/test_headers.bin".to_string(), logger_sender).unwrap();
        assert_eq!(headers.is_synced(), false);

        headers.verify_headers_sync(2000).unwrap();
        assert_eq!(headers.is_synced(), false);

        headers.verify_headers_sync(0).unwrap();
        assert_eq!(headers.is_synced(), true);

        headers.verify_headers_sync(0).unwrap();
        assert_eq!(headers.is_synced(), true);
    }
}
