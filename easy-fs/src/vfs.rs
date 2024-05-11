use alloc::{sync::Arc, vec::Vec};
use spin::{Mutex, MutexGuard};

use crate::{
    block_cache,
    block_dev::BlockDevice,
    efs::EasyFileSystem,
    layout::{DirEntry, DiskInode, DiskInodeKind, DIRENT_SIZE},
};

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a Inode
    #[inline]
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        block_cache::get(self.block_id, &self.block_device)
            .lock()
            .read(self.block_offset, f)
    }

    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        block_cache::get(self.block_id, &self.block_device)
            .lock()
            .modify(self.block_offset, f)
    }

    // Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    // Decrease the size of a disk inode
    fn decrease_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size >= disk_inode.size {
            return;
        }
        disk_inode
            .decrease_size(new_size, &self.block_device)
            .into_iter()
            .for_each(|block_id| fs.dealloc_data(block_id));
    }

    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SIZE;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SIZE * i, dirent.as_mut_bytes(), &self.block_device),
                DIRENT_SIZE,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number());
            }
        }
        None
    }

    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.disk_inode_position(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    /// Create inode under current inode by name
    pub fn create_inode(&self, name: &str, kind: DiskInodeKind) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();

        let op = |dir_inode: &DiskInode| {
            // assert it is a directory
            assert!(dir_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, dir_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }

        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.disk_inode_position(new_inode_id);
        block_cache::get(new_inode_block_id as usize, &self.block_device)
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.init(kind);
            });

        self.modify_disk_inode(|dir_inode| {
            // append file in the dirent
            let file_count = (dir_inode.size as usize) / DIRENT_SIZE;
            let new_size = (file_count + 1) * DIRENT_SIZE;
            // increase size
            self.increase_size(new_size as u32, dir_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            dir_inode.write_at(
                file_count * DIRENT_SIZE,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.disk_inode_position(new_inode_id);
        block_cache::sync_all();

        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// Create regular file under current inode
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        self.create_inode(name, DiskInodeKind::File)
    }

    /// Create directory under current inode
    pub fn create_dir(&self, name: &str) -> Option<Arc<Inode>> {
        let inode = self.create_inode(name, DiskInodeKind::Directory)?;
        inode.set_default_dirent(self.inode_id());
        Some(inode)
    }

    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert_eq!(
                data_blocks_dealloc.len(),
                DiskInode::count_total_block(size) as usize
            );
            for &data_block in &data_blocks_dealloc {
                fs.dealloc_data(data_block);
            }
        });
        block_cache::sync_all();
    }

    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            assert!(disk_inode.is_file());
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache::sync_all();
        size
    }

    /// Delete inode by name
    pub fn delete(&self, name: &str) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|dir_inode| {
            assert!(dir_inode.is_dir());
            self.find_inode_id(name, dir_inode);

            let mut last_dirent = DirEntry::empty();
            dir_inode.read_at(
                dir_inode.size as usize - DIRENT_SIZE,
                last_dirent.as_mut_bytes(),
                &self.block_device,
            );

            let file_count = (dir_inode.size as usize) / DIRENT_SIZE;
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                dir_inode.read_at(i * DIRENT_SIZE, dirent.as_mut_bytes(), &self.block_device);
                if dirent.name() == name {
                    // delete
                    fs.dealloc_inode(dirent.inode_number());
                    dir_inode.write_at(i * DIRENT_SIZE, last_dirent.as_bytes(), &self.block_device);
                    let new_size = (file_count - 1) * DIRENT_SIZE;
                    self.decrease_size(new_size as u32, dir_inode, &mut fs);
                }
            }
        });
    }

    /// Set the default `DirEntry` for the current file
    pub fn set_default_dirent(&self, parent_inode_id: u32) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|cur_dir_inode| {
            // increase size
            self.increase_size(2 * DIRENT_SIZE as u32, cur_dir_inode, &mut fs);
            // write . dirent
            let dirent_self = DirEntry::new(
                ".",
                fs.disk_inode_id(self.block_id as u32, self.block_offset),
            );
            cur_dir_inode.write_at(0, dirent_self.as_bytes(), &self.block_device);

            // write .. dirent
            let dirent_parent = DirEntry::new("..", parent_inode_id);
            cur_dir_inode.write_at(DIRENT_SIZE, dirent_parent.as_bytes(), &self.block_device);
        });
    }

    /// Get `inode_id`
    #[inline]
    pub fn inode_id(&self) -> u32 {
        self.fs
            .lock()
            .disk_inode_id(self.block_id as u32, self.block_offset)
    }

    /// Get file size
    #[inline]
    pub fn file_size(&self) -> u32 {
        self.read_disk_inode(|disk_inode| disk_inode.size)
    }

    /// Whether this inode is a directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(super::layout::DiskInode::is_dir)
    }

    /// Whether this inode is a file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.read_disk_inode(super::layout::DiskInode::is_file)
    }
}
