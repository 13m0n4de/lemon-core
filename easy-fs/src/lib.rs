//! Easy file system

#![no_std]
#![deny(missing_docs)]
#![deny(warnings)]

extern crate alloc;

mod block_cache;
mod block_dev;

/// Use a block size of 512 bytes
pub const BLOCK_SIZE: usize = 512;
/// Use a block cache of 16 blocks
pub const BLOCK_CACHE_SIZE: usize = 16;
