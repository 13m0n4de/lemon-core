use alloc::sync::Arc;

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    config::{BLOCK_SIZE, EFS_MAGIC, INDIRECT1_BOUND, INODE_DIRECT_COUNT, INODE_INDIRECT1_COUNT},
};

/// A indirect block
type IndirectBlock = [u32; BLOCK_SIZE / 4];

/// Super block of a filesystem
#[repr(C)]
pub struct SuperBlock {
    magic: u32,
    pub total_blocks: u32,
    pub inode_bitmap_blocks: u32,
    pub inode_area_blocks: u32,
    pub data_bitmap_blocks: u32,
    pub data_area_blocks: u32,
}

impl SuperBlock {
    /// Initialize a super block
    pub fn initialize(
        &mut self,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
        inode_area_blocks: u32,
        data_bitmap_blocks: u32,
        data_area_blocks: u32,
    ) {
        *self = Self {
            magic: EFS_MAGIC,
            total_blocks,
            inode_bitmap_blocks,
            inode_area_blocks,
            data_bitmap_blocks,
            data_area_blocks,
        }
    }

    /// Check if a super block is valid using efs magic
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

/// Type of a disk inode
#[derive(PartialEq)]
pub enum DiskInodeType {
    File,
    Directory,
}

/// A disk inode
#[repr(C)]
pub struct DiskInode {
    pub size: u32,
    _type: DiskInodeType,
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
}

impl DiskInode {
    /// Initialize a disk inode
    pub fn initialize(&mut self, inode_type: DiskInodeType) {
        self.size = 0;
        self._type = inode_type;
        self.direct.fill(0);
        self.indirect1 = 0;
        self.indirect2 = 0;
    }

    /// Whether this inode is a directory
    pub fn is_dir(&self) -> bool {
        self._type == DiskInodeType::Directory
    }

    /// Whether this inode is a file
    pub fn is_file(&self) -> bool {
        self._type == DiskInodeType::File
    }

    pub fn get_block_id(&self, inner_id: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let inner_id = inner_id as usize;
        if inner_id < INODE_DIRECT_COUNT {
            self.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            let indirect1 = get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[last / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1 as usize, Arc::clone(block_device))
                .lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[last % INODE_INDIRECT1_COUNT]
                })
        }
    }
}
