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

struct BlocksIBDStats {
    blocks_downloaded: usize,
    checkpoint_timestamp: u128,
    checkpoint_percentage: usize,
    checkpoint_downloads: u128,
}

pub struct BlocksState {
    ibd_stats: Option<BlocksIBDStats>,
    store_path: String,
    logger_sender: Sender<Log>,
    pub pending_blocks_ref: Arc<Mutex<PendingBlocks>>,
    sync: bool,
}

impl BlocksState {
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
                Log::Message(format!("New block received",)),
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

    pub fn get_block(&self, block_string_hash: String) -> Result<Block, CustomError> {
        let path = format!("{}/blocks/{}.bin", self.store_path, block_string_hash);
        Block::restore(path)
    }

    pub fn is_synced(&self) -> bool {
        self.sync
    }
}
