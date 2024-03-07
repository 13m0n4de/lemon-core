use alloc::sync::Arc;

use crate::{block_cache::get_block_cache, block_dev::BlockDevice, config::BLOCK_SIZE};

/// A bitmap block
type BitmapBlock = [u64; 64];

/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SIZE * 8;

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// Allocate a new block from a block device
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let pos = get_block_cache(self.start_block_id + block_id, Arc::clone(block_device))
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    match bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, &bit64)| bit64 != u64::MAX)
                        .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                    {
                        Some((bit64_pos, inner_pos)) => {
                            bitmap_block[bit64_pos] |= 1u64 << inner_pos;
                            Some(block_id * BLOCK_BITS + bit64_pos * 64 + inner_pos as usize)
                        }
                        None => None,
                    }
                });
            if pos.is_some() {
                return pos;
            }
        }
        None
    }
}
