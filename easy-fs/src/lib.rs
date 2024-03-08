//! Easy file system

#![no_std]
#![deny(missing_docs)]
#![deny(warnings)]

extern crate alloc;

mod bitmap;
mod block_cache;
mod block_dev;
mod config;
mod efs;
mod layout;
mod vfs;
