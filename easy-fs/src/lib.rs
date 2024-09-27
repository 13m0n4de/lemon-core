//! Easy file system

#![no_std]
#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

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
