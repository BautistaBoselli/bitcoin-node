use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::get_headers::GetHeaders,
    parser::BufferParser,
    peer::GENESIS,
    structs::block_header::BlockHeader,
    utils::{
        calculate_index_from_timestamp, get_current_timestamp, get_current_timestamp_millis,
        open_new_file,
    },
};

use super::utxo_state::START_DATE_IBD;

struct HeaderIBDStats {
    checkpoint_timestamp: u128,
    checkpoint_percentage: u64,
    checkpoint_downloads: u128,
}

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
    ibd_stats: Option<HeaderIBDStats>,
    sync: bool,
}

impl HeadersState {
    /// Inicializa los headers del nodo.
    /// Si el archivo donde se guardan los headers no existe, se crea.
    /// Si el archivo existe, se restauran los headers.
    pub fn new(path: String, logger_sender: Sender<Log>) -> Result<Self, CustomError> {
        let mut headers = Self {
            headers: Vec::new(),
            logger_sender: logger_sender.clone(),
            path,
            ibd_stats: None,
            sync: false,
        };

        headers.restore()?;

        send_log(
            &logger_sender,
            Log::Message(format!("Total headers restored: {}", headers.len())),
        );
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
            let header = BlockHeader::parse_from_backup(parser.extract_buffer(112)?.to_vec())?;
            self.headers.push(header);
        }

        Ok(())
    }

    fn save(&self, headers: &Vec<BlockHeader>) -> Result<(), CustomError> {
        let mut file = open_new_file(self.path.clone(), true)?;
        let mut buffer = vec![];
        for header in headers {
            let header_buffer: Vec<u8> = header.serialize_for_backup();
            buffer.extend(header_buffer);
        }

        file.write_all(buffer.as_slice())?;
        Ok(())
    }

    fn len(&self) -> usize {
        self.headers.len()
    }

    pub fn total_headers_to_download(&self) -> usize {
        self.len() - calculate_index_from_timestamp(&self.headers, START_DATE_IBD)
    }

    /// Devuelve todos los headers del nodo.
    pub fn get_all(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn get_header_index(&self, block_hash: &Vec<u8>) -> usize {
        let position_from_end = self
            .headers
            .iter()
            .rev()
            .position(|header| header.hash() == block_hash);

        if let Some(position_from_end) = position_from_end {
            return self.headers.len() - position_from_end - 1;
        }

        0
    }

    /// Devuelve el hash del ultimo header del nodo.
    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.last().map(|header| header.hash().clone())
    }

    /// Devuelve los ultimos count headers del nodo junto a su height.
    pub fn get_last_headers(&self, count: usize) -> Vec<(usize, BlockHeader)> {
        let mut last_headers = vec![];

        self.headers
            .iter()
            .enumerate()
            .rev()
            .take(count)
            .for_each(|(index, header)| last_headers.push((index + 1, header.clone())));

        last_headers
    }

    /// Agrega los headers al nodo y los almacena.
    /// Tambien verifica si con los nuevos queda sincronizado con la red
    pub fn append_headers(&mut self, mut headers: Vec<BlockHeader>) -> Result<(), CustomError> {
        if let Some(first_header) = headers.first() {
            let last_header = self.headers.last();
            let last_header_hash = last_header
                .map(|header| header.hash().clone())
                .unwrap_or(GENESIS.to_vec());

            if last_header_hash != first_header.prev_block_hash {
                return Err(CustomError::BlockChainBroken);
            }

            let percentage = self.calculate_percentage_downloaded(first_header.timestamp)?;
            if self.ibd_stats.is_none() && percentage < 95_u64 {
                self.start_stats_printing()?;
            }
        }

        self.save(&headers)?;
        let headers_count = headers.len();
        self.headers.append(&mut headers);

        self.print_status(headers_count)?;
        self.verify_headers_sync(headers_count)?;
        Ok(())
    }

    fn calculate_percentage_downloaded(&self, received_timestamp: u32) -> Result<u64, CustomError> {
        let first_timestamp = self
            .headers
            .first()
            .map(|header| header.timestamp)
            .unwrap_or(received_timestamp) as u64;

        let now = get_current_timestamp()?;

        if now - first_timestamp > 0 {
            Ok((received_timestamp as u64 - first_timestamp) * 100 / (now - first_timestamp))
        } else {
            Ok(0)
        }
    }

    fn start_stats_printing(&mut self) -> Result<(), CustomError> {
        self.ibd_stats = Some(HeaderIBDStats {
            checkpoint_timestamp: get_current_timestamp_millis()?,
            checkpoint_percentage: 0,
            checkpoint_downloads: 0,
        });

        Ok(())
    }

    fn print_status(&mut self, headers_count: usize) -> Result<(), CustomError> {
        if self.is_synced() || self.ibd_stats.is_none() {
            send_log(
                &self.logger_sender,
                Log::Message(format!(
                    "New headers: {}, total {}",
                    headers_count,
                    self.headers.len()
                )),
            );
        } else {
            self.print_stats(headers_count)?;
        }

        Ok(())
    }

    fn print_stats(&mut self, headers_count: usize) -> Result<(), CustomError> {
        let last_timestamp = self.headers.last().map(|h| h.timestamp).unwrap_or(0);
        let percentage = self.calculate_percentage_downloaded(last_timestamp)?;

        if let Some(ibd_stats) = &mut self.ibd_stats {
            ibd_stats.checkpoint_downloads += headers_count as u128;

            let now = get_current_timestamp_millis()?;

            if percentage > ibd_stats.checkpoint_percentage {
                let checkpoint_time = now - ibd_stats.checkpoint_timestamp;
                let headers_per_second = if ibd_stats.checkpoint_percentage > 0 {
                    ibd_stats.checkpoint_downloads * 1000 / checkpoint_time
                } else {
                    0
                };
                send_log(
                    &self.logger_sender,
                    Log::Message(format!(
                        "Headers sync {}% at {} headers/s... total {}",
                        percentage,
                        headers_per_second,
                        self.headers.len(),
                    )),
                );

                ibd_stats.checkpoint_downloads = 0;
                ibd_stats.checkpoint_percentage = percentage;
                ibd_stats.checkpoint_timestamp = now;
            }
        }

        Ok(())
    }

    pub fn set_downloaded(&mut self, block_hash: &Vec<u8>) {
        let downloaded_block = self
            .headers
            .iter_mut()
            .rev()
            .find(|header| header.hash() == block_hash);

        if let Some(header) = downloaded_block {
            header.block_downloaded = true;
        }
    }

    pub fn get_headers_to_send(&mut self, block_hash: &Vec<u8>) -> Vec<BlockHeader> {
        let downloaded_block_index = self.get_header_index(block_hash);

        if downloaded_block_index == 0 {
            return vec![];
        }

        let downloaded_block_prev = self.headers[downloaded_block_index - 1].clone();

        let mut headers_to_send = vec![];
        if downloaded_block_prev.broadcasted {
            for header in self.headers.iter_mut().skip(downloaded_block_index) {
                if header.block_downloaded {
                    headers_to_send.push(header.clone());
                    header.broadcasted = true;
                }
            }
        }

        println!("Hola, headers_to_send from: {:?}", downloaded_block_index);
        println!("Hola, headers_to_send len: {:?}", headers_to_send.len());

        headers_to_send
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
        let peer_last_header = get_headers
            .block_locator_hashes
            .last()
            .unwrap_or(&GENESIS.to_vec())
            .clone();
        if let Some(last_header) = self.headers.last() {
            if peer_last_header == *last_header.hash() {
                return vec![];
            }
        }

        if peer_last_header == GENESIS.to_vec() {
            return self.first_headers(get_headers.hash_stop);
        }

        self.get_requested_headers(peer_last_header, get_headers.hash_stop)
    }

    fn get_requested_headers(
        &self,
        peer_last_header: Vec<u8>,
        hash_stop: Vec<u8>,
    ) -> Vec<BlockHeader> {
        let mut headers = vec![];
        let mut found = false;
        for header in &self.headers {
            if header.prev_block_hash == peer_last_header {
                found = true;
            }
            if found {
                headers.push(header.clone());
            }
            if headers.len() == 2000 || *header.hash() == hash_stop {
                break;
            }
        }

        if !found {
            return self.first_headers(hash_stop);
        }
        headers
    }

    fn first_headers(&self, hash_stop: Vec<u8>) -> Vec<BlockHeader> {
        self.headers
            .iter()
            .take(2000)
            .take_while(|block| block.hash != hash_stop)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use std::{
        fs::{self, remove_file},
        sync::mpsc,
    };

    use crate::messages::headers::Headers;

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
        println!("headers: {:?}", headers.headers.last().unwrap());

        let mut new_headers = Headers::new();
        new_headers.headers.push(BlockHeader {
            prev_block_hash: vec![
                32, 120, 42, 0, 82, 85, 182, 87, 105, 110, 160, 87, 213, 185, 143, 52, 222, 252,
                247, 81, 150, 246, 79, 110, 234, 200, 2, 108, 0, 0, 0, 0,
            ],
            merkle_root: vec![],
            version: 0,
            timestamp: 0,
            bits: 0,
            nonce: 0,
            hash: vec![],
            block_downloaded: true,
            broadcasted: true,
        });

        headers.append_headers(new_headers.headers).unwrap();
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
