use crate::config::{EFS_MAGIC, INODE_DIRECT_COUNT};

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
}
