//! File system

mod inode;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;
use alloc::sync::Arc;
pub use inode::{find_inode, get_full_path, open_file, OpenFlags};
pub use pipe::make_pipe;
pub use stdio::{Stdin, Stdout};

const CHR: usize = 0;
const REG: usize = 1;
const DIR: usize = 2;
const LNK: usize = 3;

/// File trait
pub trait File: Send + Sync {
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;
    /// If readable
    fn is_readable(&self) -> bool;
    /// If writable
    fn is_writable(&self) -> bool;
    fn offset(&self) -> usize {
        0
    }
    #[allow(unused)]
    fn set_offset(&self, _offset: usize) {}
    fn file_size(&self) -> usize {
        0
    }
    fn inode_id(&self) -> usize {
        0
    }
    fn mode(&self) -> usize {
        CHR
    }
}

pub struct Stat {
    pub ino: u32,
    pub mode: u32,
    pub off: u32,
    pub size: u32,
}

impl From<Arc<dyn File + Send + Sync>> for Stat {
    fn from(file: Arc<dyn File + Send + Sync>) -> Self {
        Self {
            ino: file.inode_id() as u32,
            mode: file.mode() as u32,
            off: file.offset() as u32,
            size: file.file_size() as u32,
        }
    }
}
