use std::{
    fs::read_dir,
    sync::{mpsc::Sender, Arc, Mutex},
};

use crate::{
    error::CustomError,
    logger::{send_log, Log},
    messages::block::Block,
    utils::get_current_timestamp_millis,
};

use super::pending_blocks_state::PendingBlocks;

/// BlocksIBDStats es una estructura que contiene los elementos necesarios para manejar las
/// estadisticas de la descarga masiva de bloques.
/// Solamente se utiliza cuando la cantidad de bloques a descargar
/// es mayor al 2% de los headers posteriores al START_DATE_IBD.
/// Los elementos son:
/// - blocks_downloaded: Cantidad de bloques totales descargados.
/// - checkpoint_timestamp: Timestamp del ultimo checkpoint.
/// - checkpoint_percentage: Ultimo porcentaje alcanzado de la descarga de bloques
///  en proporcion al total de los mismos.
/// - checkpoint_downloads: Cantidad de bloques descargados desde el ultimo checkpoint.
struct BlocksIBDStats {
    blocks_downloaded: usize,
    checkpoint_timestamp: u128,
    checkpoint_percentage: usize,
    checkpoint_downloads: u128,
}

/// BlocksState es una estructura que contiene los elementos necesarios para manejar los bloques.
/// Los elementos son:
/// - ibd_stats: Option<BLocksIBDStats> solamente se inicializa cuando corresponde.
/// - store_path: Path de la carpeta donde se crea el directorio donde se encuentran los bloques.
/// - logger_sender: Sender para enviar logs al logger.
/// - pending_blocks_ref: Referencia a los bloques pendientes.
/// - sync: Booleano que indica si el nodo esta sincronizado.
pub struct BlocksState {
    ibd_stats: Option<BlocksIBDStats>,
    store_path: String,
    logger_sender: Sender<Log>,
    pub pending_blocks_ref: Arc<Mutex<PendingBlocks>>,
    sync: bool,
}

impl BlocksState {
    /// Inicializa el estado de los bloques.
    pub fn new(
        store_path: String,
        logger_sender: Sender<Log>,
        pending_blocks_ref: Arc<Mutex<PendingBlocks>>,
    ) -> Self {
        Self {
            ibd_stats: None,
            pending_blocks_ref,
            store_path,
            logger_sender,
            sync: false,
        }
    }

    /// Se encarga de guardar en disco el bloque y eliminarlo de los bloques pendientes.
    /// Si la cantidad de bloques a descargar es mayor al 2% de los headers posteriores al START_DATE_IBD
    /// comienza los stats de la descarga.
    pub fn append_block(
        &mut self,
        block_hash: &Vec<u8>,
        block: &Block,
        total_blocks: usize,
    ) -> Result<(), CustomError> {
        let path = format!(
            "{}/blocks/{}.bin",
            self.store_path,
            block.header.hash_as_string()
        );
        block.save(path)?;

        if self.ibd_stats.is_none() {
            let blocks_downloaded = read_dir(format!("{}/blocks", self.store_path))?.count();
            let percentage = (blocks_downloaded * 100) / total_blocks;

            if percentage < 98_usize {
                self.ibd_stats = Some(BlocksIBDStats {
                    blocks_downloaded,
                    checkpoint_timestamp: block.header.timestamp as u128 * 1000,
                    checkpoint_percentage: percentage,
                    checkpoint_downloads: 0,
                })
            }
        }

        self.print_status(total_blocks)?;

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        pending_blocks.remove_block(block_hash)?;
        drop(pending_blocks);

        Ok(())
    }

    fn print_status(&mut self, total_blocks: usize) -> Result<(), CustomError> {
        if self.is_synced() || self.ibd_stats.is_none() {
            send_log(
                &self.logger_sender,
                Log::Message(String::from("New block received")),
            );
        } else {
            self.print_stats(total_blocks)?;
        }

        Ok(())
    }

    fn print_stats(&mut self, total_blocks: usize) -> Result<(), CustomError> {
        if let Some(ibd_stats) = &mut self.ibd_stats {
            ibd_stats.blocks_downloaded += 1;
            ibd_stats.checkpoint_downloads += 1;

            let percentage = (ibd_stats.blocks_downloaded * 100) / total_blocks;
            if percentage > ibd_stats.checkpoint_percentage {
                let now = get_current_timestamp_millis()?;
                let checkpoint_time = now - ibd_stats.checkpoint_timestamp;
                let blocks_per_second = ibd_stats.checkpoint_downloads * 1000 / checkpoint_time;

                send_log(
                    &self.logger_sender,
                    Log::Message(format!(
                        "Blocks sync {}% at {} blocks/s... total {}",
                        percentage, blocks_per_second, ibd_stats.blocks_downloaded
                    )),
                );

                ibd_stats.checkpoint_percentage = percentage;
                ibd_stats.checkpoint_timestamp = now;
                ibd_stats.checkpoint_downloads = 0;
            }
        }

        Ok(())
    }

    /// Verifica si los bloques estan sincronizado.
    /// Para esto se necesita que no haya bloques pendientes.
    pub fn verify_sync(&mut self) -> Result<(), CustomError> {
        if self.sync {
            return Ok(());
        }

        let mut pending_blocks = self.pending_blocks_ref.lock()?;
        self.sync = pending_blocks.is_empty();

        if self.sync {
            pending_blocks.drain();
            send_log(
                &self.logger_sender,
                Log::Message("blocks sync completed".to_string()),
            );
        }
        Ok(())
    }

    /// Devuelve el bloque correspondiente al hash pasado por parametro.
    pub fn get_block(&self, block_string_hash: String) -> Result<Block, CustomError> {
        let path = format!("{}/blocks/{}.bin", self.store_path, block_string_hash);
        Block::restore(path)
    }

    /// Retorna el estado de sincronizacion de los bloques.
    pub fn is_synced(&self) -> bool {
        self.sync
    }
}

#[cfg(test)]
mod tests {

    use std::{fs, path::Path, sync::mpsc};

    use super::*;

    #[test]
    fn blocks_state_append() {
        let store_path = "tests".to_string();
        let (logger_sender, _) = mpsc::channel();
        let pending_blocks_ref = PendingBlocks::new(&store_path, &vec![]);
        let mut blocks_state =
            BlocksState::new(store_path.clone(), logger_sender, pending_blocks_ref);

        let mut pending = blocks_state.pending_blocks_ref.lock().unwrap();
        pending.append_block(vec![1, 2, 3]).unwrap();
        drop(pending);

        let mut block = blocks_state.get_block("test_block".to_string()).unwrap();
        block.header.hash = vec![1, 2, 3];

        blocks_state
            .append_block(&vec![1, 2, 3], &block, 1)
            .unwrap();

        let pending = blocks_state.pending_blocks_ref.lock().unwrap();
        assert_eq!(pending.is_empty(), true);

        assert!(Path::new(&format!("{}/blocks/010203.bin", store_path)).exists());
        fs::remove_file(format!("{}/blocks/010203.bin", store_path)).unwrap();
    }

    #[test]
    fn blocks_state_verify_sync() {
        let store_path = "tests".to_string();
        let (logger_sender, _) = mpsc::channel();
        let pending_blocks_ref = PendingBlocks::new(&store_path, &vec![]);
        let mut blocks_state =
            BlocksState::new(store_path.clone(), logger_sender, pending_blocks_ref);

        let mut pending = blocks_state.pending_blocks_ref.lock().unwrap();
        pending.append_block(vec![1, 2, 3]).unwrap();
        drop(pending);

        assert_eq!(blocks_state.is_synced(), false);
        blocks_state.verify_sync().unwrap();
        assert_eq!(blocks_state.is_synced(), false);

        let mut pending = blocks_state.pending_blocks_ref.lock().unwrap();
        pending.remove_block(&vec![1, 2, 3]).unwrap();
        drop(pending);

        assert_eq!(blocks_state.is_synced(), false);
        blocks_state.verify_sync().unwrap();
        assert_eq!(blocks_state.is_synced(), true);

        let mut pending = blocks_state.pending_blocks_ref.lock().unwrap();
        pending.append_block(vec![1, 2, 3]).unwrap();
        drop(pending);

        assert_eq!(blocks_state.is_synced(), true);
        blocks_state.verify_sync().unwrap();
        assert_eq!(blocks_state.is_synced(), true);
    }
}
