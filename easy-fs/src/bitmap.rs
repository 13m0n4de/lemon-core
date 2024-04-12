use alloc::sync::Arc;

use crate::{block_cache, block_dev::BlockDevice, config::BLOCK_BITS};

/// A bitmap block
type BitmapBlock = [u64; 64];

pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

impl Bitmap {
    #[inline]
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }

    /// Allocate a new block from a block device
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {
            let id = block_cache::get(self.start_block_id + block_id, block_device)
                .lock()
                .modify(0, |bitmap_block: &mut BitmapBlock| {
                    match bitmap_block
                        .iter()
                        .enumerate()
                        .find(|(_, &bit64)| bit64 != u64::MAX)
                        .map(|(bits64_id, bits64)| (bits64_id, bits64.trailing_ones() as usize))
                    {
                        Some((bit64_id, inner_id)) => {
                            bitmap_block[bit64_id] |= 1u64 << inner_id;
                            Some(block_id * BLOCK_BITS + bit64_id * 64 + inner_id)
                        }
                        None => None,
                    }
                });
            if id.is_some() {
                return id;
            }
        }
        None
    }

    /// Deallocate a block
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_id, bits64_id, inner_id) = decomposition(bit);
        block_cache::get(self.start_block_id + block_id, block_device)
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_id] & (1u64 << inner_id) > 0);
                bitmap_block[bits64_id] &= !(1u64 << inner_id);
            });
    }

    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}

/// Decompose bits into (`block_id`, `bits64_id`, `inner_id`)
fn decomposition(bit: usize) -> (usize, usize, usize) {
    let block_id = bit / BLOCK_BITS;
    let bit_in_block = bit % BLOCK_BITS;
    let bit64_id = bit_in_block / 64;
    let inner_id = bit_in_block % 64;
    (block_id, bit64_id, inner_id)
}
