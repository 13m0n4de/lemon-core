use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::lazy_static;

use crate::{drivers::BLOCK_DEVICE, mm::UserBuffer, sync::UPIntrFreeCell};

use super::{File, StatMode};

/// A wrapper around a filesystem inode
/// to implement File trait atop
#[allow(clippy::module_name_repetitions)]
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPIntrFreeCell<OSInodeInner>,
}

/// The OS inode inner in '`UPIntrFreeCell`'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    /// Construct an OS inode from a inode
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: unsafe { UPIntrFreeCell::new(OSInodeInner { offset: 0, inode }) },
        }
    }

    /// Read all data inside a inode into vector
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.exclusive_access();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}

impl File for OSInode {
    fn is_readable(&self) -> bool {
        self.readable
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_read_size = 0usize;
        for slice in &mut buf.buffers {
            let read_size = inner.inode.read_at(inner.offset, slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }

    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.exclusive_access();
        let mut total_write_size = 0usize;
        for slice in &buf.buffers {
            let write_size = inner.inode.write_at(inner.offset, slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }

    fn offset(&self) -> usize {
        self.inner.exclusive_access().offset
    }

    fn set_offset(&self, offset: usize) {
        self.inner.exclusive_access().offset = offset;
    }

    fn file_size(&self) -> u32 {
        self.inner.exclusive_access().inode.file_size()
    }

    fn inode_id(&self) -> u32 {
        self.inner.exclusive_access().inode.inode_id()
    }

    fn mode(&self) -> StatMode {
        let inode = &self.inner.exclusive_access().inode;
        if inode.is_file() {
            StatMode::REG
        } else if inode.is_dir() {
            StatMode::DIR
        } else {
            StatMode::LNK
        }
    }
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(&BLOCK_DEVICE);
        let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
        root_inode.set_default_dirent(root_inode.inode_id());
        root_inode
    };
    pub static ref PROC_INODE: Arc<Inode> = {
        ROOT_INODE.delete("proc");
        let proc_inode = ROOT_INODE
            .create_dir("proc")
            .expect("Failed to create inode for '/proc/'.");
        proc_inode.set_default_dirent(ROOT_INODE.inode_id());
        proc_inode
    };
}

bitflags! {
    /// Open file flags
    pub struct OpenFlags: u32 {
        /// Read only
        const RDONLY = 0;
        /// Write only
        const WRONLY = 1;
        /// Read & Write
        const RDWR = 1 << 1;
        /// Allow create
        const CREATE = 1 << 9;
        /// Clear file and return an empty one
        const TRUNC = 1 << 10;
    }
}

/// Finding an inode using an absolute path
pub fn find(path: &str) -> Option<Arc<Inode>> {
    path.split('/').try_fold(ROOT_INODE.clone(), |node, name| {
        if name.is_empty() {
            Some(node)
        } else {
            node.find(name)
        }
    })
}
