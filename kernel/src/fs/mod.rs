//! File system

mod inode;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;
use alloc::sync::Arc;
use bitflags::bitflags;
pub use inode::{find_inode, get_full_path, open_file, OpenFlags, PROC_INODE};
pub use pipe::make_pipe;
pub use stdio::{Stdin, Stdout};

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
    fn file_size(&self) -> u32 {
        0
    }
    fn inode_id(&self) -> u32 {
        0
    }
    fn mode(&self) -> StatMode {
        StatMode::NULL
    }
}

#[repr(C)]
#[derive(Default)]
pub struct Stat {
    pub dev: u32,
    pub ino: u32,
    pub mode: StatMode,
    pub off: usize,
    pub size: u32,
}

impl From<Arc<dyn File + Send + Sync>> for Stat {
    fn from(file: Arc<dyn File + Send + Sync>) -> Self {
        Self {
            dev: 0,
            ino: file.inode_id(),
            mode: file.mode(),
            off: file.offset(),
            size: file.file_size(),
        }
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Default)]
    pub struct StatMode: u32 {
        const NULL = 0;
        const DIR = 0o040000;
        const REG = 0o100000;
        const LNK = 0o120000;
    }
}
