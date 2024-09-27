//! Easy file system

#![no_std]

extern crate alloc;
extern crate log;

mod bitmap;
mod block_cache;
mod block_dev;
mod config;
mod efs;
mod layout;
mod vfs;

pub use block_dev::BlockDevice;
pub use config::BLOCK_SIZE;
pub use efs::EasyFileSystem;
pub use layout::DIRENT_SIZE;
pub use vfs::Inode;
