use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{error::CustomError, utils::get_current_timestamp};

/// PendingBlocks es una estructura para manejar los bloques solicitados pendientes de recibir.
/// Los elementos son:
/// - blocks: HashMap que contiene los bloques pendientes de recibir y un timestamp del momento en el que se generó.
/// - stale_time: Tiempo en segundos que debe pasar para que una peticion de bloque sea considerada como vencida.
pub struct PendingBlocks {
    blocks: HashMap<Vec<u8>, u64>,
    stale_time: u64,
}

impl PendingBlocks {
    #[must_use]
    /// Inicializa la estructura.
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            blocks: HashMap::new(),
            stale_time: 5,
        }))
    }

    /// Agrega un bloque a la lista de bloques pendientes de recibir con el timestamp actual.
    pub fn append_block(&mut self, block_hash: Vec<u8>) -> Result<(), CustomError> {
        let current_time = get_current_timestamp()?;
        self.blocks.insert(block_hash, current_time);
        Ok(())
    }

    /// Elimina un bloque de la lista de bloques pendientes de recibir.
    pub fn remove_block(&mut self, block_hash: &Vec<u8>) -> Result<(), CustomError> {
        self.blocks.remove(block_hash);
        Ok(())
    }

    /// Elimina todos los bloques de la lista de bloques pendientes de recibir.
    pub fn drain(&mut self) {
        self.blocks.drain();
    }

    /// Devuelve los bloques pendientes de recibir que ya vencieron.
    pub fn get_stale_requests(&mut self) -> Result<Vec<Vec<u8>>, CustomError> {
        let mut to_remove = Vec::new();

        for (block_hash, timestamp) in &self.blocks {
            if *timestamp + self.stale_time < get_current_timestamp()? {
                to_remove.push(block_hash.clone());
            }
        }

        for block_hash in &to_remove {
            self.blocks.remove(block_hash);
        }

        Ok(to_remove)
    }

    /// Devuelve true si el bloque esta en la lista de bloques pendientes de recibir.
    pub fn is_block_pending(&self, block_hash: &Vec<u8>) -> bool {
        self.blocks.contains_key(block_hash)
    }

    /// Devuelve true si la lista de bloques pendientes de recibir esta vacia.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

#[cfg(test)]
mod tests {

    use std::{thread, time::Duration};

    use super::*;

    #[test]
    fn pending_blocks_creation() {
        let pending_blocks = PendingBlocks::new();
        let pending_blocks = pending_blocks.lock().unwrap();

        assert_eq!(pending_blocks.is_empty(), true);
    }

    #[test]
    fn append_block() {
        let pending_blocks = PendingBlocks::new();
        let mut pending_blocks = pending_blocks.lock().unwrap();

        let block_hash = vec![1, 2, 3, 4, 5];
        pending_blocks.append_block(block_hash.clone()).unwrap();

        assert_eq!(pending_blocks.is_block_pending(&block_hash), true);
        assert_eq!(pending_blocks.is_empty(), false);
    }

    #[test]
    fn remove_block() {
        let pending_blocks = PendingBlocks::new();
        let mut pending_blocks = pending_blocks.lock().unwrap();

        let block_hash = vec![1, 2, 3, 4, 5];
        pending_blocks.append_block(block_hash.clone()).unwrap();
        assert_eq!(pending_blocks.is_empty(), false);
        pending_blocks.remove_block(&block_hash).unwrap();

        assert_eq!(pending_blocks.is_block_pending(&block_hash), false);
        assert_eq!(pending_blocks.is_empty(), true);
    }

    #[test]
    fn drain() {
        let pending_blocks = PendingBlocks::new();
        let mut pending_blocks = pending_blocks.lock().unwrap();

        let block_hash = vec![1, 2, 3, 4, 5];
        let block_hash2 = vec![6, 7, 8, 9, 10];
        pending_blocks.append_block(block_hash.clone()).unwrap();
        pending_blocks.append_block(block_hash2.clone()).unwrap();

        assert_eq!(pending_blocks.is_empty(), false);
        pending_blocks.drain();
        assert_eq!(pending_blocks.is_empty(), true);
    }

    #[test]
    fn get_stale_requests() {
        let pending_blocks = PendingBlocks::new();
        let mut pending_blocks = pending_blocks.lock().unwrap();

        let block_hash = vec![1, 2, 3, 4, 5];

        pending_blocks.append_block(block_hash.clone()).unwrap();

        let stale_requests = pending_blocks.get_stale_requests().unwrap();
        assert_eq!(stale_requests.len(), 0);
        pending_blocks.stale_time = 0;
        thread::sleep(Duration::from_secs(1));

        let stale_requests = pending_blocks.get_stale_requests().unwrap();
        assert_eq!(stale_requests.len(), 1);
        assert_eq!(stale_requests[0], block_hash);
    }
}
