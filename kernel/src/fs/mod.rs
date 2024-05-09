//! File system

pub mod inode;
pub mod pipe;
pub mod stdio;

use crate::mm::UserBuffer;
use alloc::{string::String, sync::Arc, vec::Vec};
use bitflags::bitflags;
use inode::OSInode;

pub use inode::{OpenFlags, PROC_INODE};
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
        const DIR = 0o040_000;
        const REG = 0o100_000;
        const LNK = 0o120_000;
    }
}

/// Calculate the absolute path of the input path
pub fn get_full_path(cwd: &str, path: &str) -> String {
    let resolved_path = if path.starts_with('/') {
        String::from(path)
    } else {
        String::from(cwd) + "/" + path
    };

    let mut parts = Vec::new();

    for part in resolved_path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => {
                parts.push(part);
            }
        }
    }

    String::from("/") + &parts.join("/")
}

/// Open file with flags
#[allow(clippy::needless_pass_by_value)]
pub fn open_file(path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let readable = flags.contains(OpenFlags::RDONLY) || flags.contains(OpenFlags::RDWR);
    let writable = flags.contains(OpenFlags::WRONLY) || flags.contains(OpenFlags::RDWR);

    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = inode::find(path) {
            if inode.is_file() {
                // clear size
                inode.clear();
            }
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            let (parent_path, target) = match path.rsplit_once('/') {
                Some((parent_path, target)) => (parent_path, target),
                None => ("", path),
            };
            let parent_inode = inode::find(parent_path)?;
            parent_inode
                .create(target)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        inode::find(path).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}
