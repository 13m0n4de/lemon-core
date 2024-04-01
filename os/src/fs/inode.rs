use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use easy_fs::{EasyFileSystem, Inode};
use lazy_static::lazy_static;

use crate::{drivers::BLOCK_DEVICE, mm::UserBuffer, sync::UPSafeCell};

use super::File;

/// A wrapper around a filesystem inode
/// to implement File trait atop
#[allow(clippy::module_name_repetitions)]
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: UPSafeCell<OSInodeInner>,
}

/// The OS inode inner in '`UPSafeCell`'
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
            inner: unsafe { UPSafeCell::new(OSInodeInner { offset: 0, inode }) },
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
}

lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(&BLOCK_DEVICE);
        Arc::new(EasyFileSystem::root_inode(&efs))
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

/// List all files in the filesystem
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

/// Open file with flags
#[allow(clippy::needless_pass_by_value)]
pub fn open_file(name: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {
    let readable = flags.contains(OpenFlags::RDONLY) || flags.contains(OpenFlags::RDWR);
    let writable = flags.contains(OpenFlags::WRONLY) || flags.contains(OpenFlags::RDWR);

    if flags.contains(OpenFlags::CREATE) {
        if let Some(inode) = ROOT_INODE.find(name) {
            // clear size
            inode.clear();
            Some(Arc::new(OSInode::new(readable, writable, inode)))
        } else {
            // create file
            ROOT_INODE
                .create(name)
                .map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        ROOT_INODE.find(name).map(|inode| {
            if flags.contains(OpenFlags::TRUNC) {
                inode.clear();
            }
            Arc::new(OSInode::new(readable, writable, inode))
        })
    }
}
