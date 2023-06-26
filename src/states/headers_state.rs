use std::{
    io::{Read, Write},
    sync::mpsc::Sender,
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::headers::Headers,
    parser::BufferParser,
    structs::block_header::BlockHeader,
    utils::open_new_file,
};

pub struct HeadersState {
    headers: Vec<BlockHeader>,
    logger_sender: Sender<Log>,
    path: String,
    sync: bool,
}

impl HeadersState {
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
        if parser.len() % 80 != 0 {
            return Err(CustomError::SerializedBufferIsInvalid);
        }

        while !parser.is_empty() {
            let header = BlockHeader::parse(parser.extract_buffer(80)?.to_vec(), false)?;
            self.headers.push(header);
        }
        Ok(())
    }

    pub fn get_all(&self) -> &Vec<BlockHeader> {
        &self.headers
    }

    pub fn get_last_header_hash(&self) -> Option<Vec<u8>> {
        self.headers.last().map(|header| header.hash())
    }

    pub fn append_headers(&mut self, headers: &mut Headers) -> Result<(), CustomError> {
        let mut file = open_new_file(self.path.clone(), true)?;

        let mut buffer = vec![];
        for header in &headers.headers {
            let header_buffer: Vec<u8> = header.serialize();
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

    pub fn is_synced(&self) -> bool {
        self.sync
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
