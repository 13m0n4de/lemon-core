use alloc::{sync::Arc, vec::Vec};

use crate::{
    block_cache::get_block_cache,
    block_dev::BlockDevice,
    config::{
        BLOCK_SIZE, DIRECT_BOUND, EFS_MAGIC, INDIRECT1_BOUND, INODE_DIRECT_COUNT,
        INODE_INDIRECT1_COUNT, INODE_INDIRECT2_COUNT, NAME_LENGTH_LIMIT,
    },
};

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

/// A indirect block
type IndirectBlock = [u32; BLOCK_SIZE / 4];
/// A data block
pub type DataBlock = [u8; BLOCK_SIZE];

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
    #[allow(unused)]
    pub fn is_file(&self) -> bool {
        self._type == DiskInodeType::File
    }

    /// Get id of block given inner id
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

    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SIZE as u32 - 1) / BLOCK_SIZE as u32
    }

    /// Return block number correspond to size.
    pub fn data_blocks(&self) -> u32 {
        Self::_data_blocks(self.size)
    }

    /// Return number of blocks needed include indirect1/2.
    pub fn total_blocks(size: u32) -> u32 {
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = Self::_data_blocks(size) as usize;
        // indirect1
        if data_blocks > DIRECT_BOUND {
            total += 1;
        }
        // indirect2
        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            let indirect1_needed =
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
            total += indirect1_needed;
        }
        total as u32
    }

    /// Get the number of data blocks that have to be allocated given the new size of data
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::total_blocks(new_size) - Self::total_blocks(self.size)
    }

    /// Increase the size of current disk inode
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut current_blocks = self.data_blocks() as usize;
        self.size = new_size;
        let mut total_blocks = self.data_blocks() as usize;
        let mut new_blocks = new_blocks.into_iter();

        // fill direct
        let direct_fill_count = total_blocks.min(INODE_DIRECT_COUNT);
        while current_blocks < direct_fill_count {
            self.direct[current_blocks] = new_blocks.next().unwrap();
            current_blocks += 1;
        }

        // alloc indirect1
        if total_blocks > INODE_DIRECT_COUNT {
            if current_blocks == INODE_DIRECT_COUNT {
                self.indirect1 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_DIRECT_COUNT;
            total_blocks -= INODE_DIRECT_COUNT;
        } else {
            return;
        }
        // fill indirect1
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                let indirect1_fill_count = total_blocks.min(INODE_INDIRECT1_COUNT);
                while current_blocks < indirect1_fill_count {
                    indirect1[current_blocks] = new_blocks.next().unwrap();
                    current_blocks += 1;
                }
            });

        // alloc indirect2
        if total_blocks > INODE_INDIRECT1_COUNT {
            if current_blocks == INODE_INDIRECT1_COUNT {
                self.indirect2 = new_blocks.next().unwrap();
            }
            current_blocks -= INODE_INDIRECT1_COUNT;
            total_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return;
        }
        // fill indirect2 from (a0, b0) -> (a1, b1)
        let mut a0 = current_blocks / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks % INODE_INDIRECT1_COUNT;
        let a1 = total_blocks / INODE_INDIRECT1_COUNT;
        let b1 = total_blocks % INODE_INDIRECT1_COUNT;
        // alloc low-level indirect1
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    if b0 == 0 {
                        indirect2[a0] = new_blocks.next().unwrap();
                    }
                    // fill current
                    get_block_cache(indirect2[a0] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            indirect1[b0] = new_blocks.next().unwrap();
                        });
                    // move to next
                    b0 += 1;
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });
    }

    ///
    pub fn decrease_size(
        &mut self,
        new_size: u32,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let mut current_blocks = self.data_blocks() as usize;
        self.size = new_size;
        let mut recycled_blocks = self.data_blocks() as usize;

        // recycle direct
        let direct_recycle_count = current_blocks.min(INODE_DIRECT_COUNT);
        while current_blocks < direct_recycle_count {
            v.push(self.direct[recycled_blocks]);
            self.direct[recycled_blocks] = 0;
            recycled_blocks += 1;
        }

        // recycle indirect1
        if recycled_blocks > INODE_DIRECT_COUNT {
            if current_blocks == INODE_DIRECT_COUNT {
                v.push(self.indirect1);
                self.indirect1 = 0;
            }
            current_blocks -= INODE_DIRECT_COUNT;
            recycled_blocks -= INODE_DIRECT_COUNT;
        } else {
            return v;
        }
        // fill indirect1
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                let indirect1_recycle_count = recycled_blocks.min(INODE_INDIRECT1_COUNT);
                while current_blocks < indirect1_recycle_count {
                    v.push(indirect1[current_blocks]);
                    current_blocks += 1;
                }
            });

        // alloc indirect2
        if recycled_blocks > INODE_INDIRECT1_COUNT {
            if current_blocks == INODE_INDIRECT1_COUNT {
                v.push(self.indirect2);
                self.indirect2 = 0;
            }
            current_blocks -= INODE_INDIRECT1_COUNT;
            recycled_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        // fill indirect2 from (a0, b0) -> (a1, b1)
        let mut a0 = current_blocks / INODE_INDIRECT1_COUNT;
        let mut b0 = current_blocks % INODE_INDIRECT1_COUNT;
        let a1 = recycled_blocks / INODE_INDIRECT1_COUNT;
        let b1 = recycled_blocks % INODE_INDIRECT1_COUNT;
        // alloc low-level indirect1
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                while (a0 < a1) || (a0 == a1 && b0 < b1) {
                    if b0 == 0 {
                        v.push(indirect2[a0]);
                    }
                    // fill current
                    get_block_cache(indirect2[a0] as usize, Arc::clone(block_device))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            v.push(indirect1[b0]);
                        });
                    // move to next
                    b0 += 1;
                    if b0 == INODE_INDIRECT1_COUNT {
                        b0 = 0;
                        a0 += 1;
                    }
                }
            });

        v
    }

    /// Clear size to zero and return blocks that should be deallocated.
    /// We will clear the block contents to zero later.
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let mut data_blocks = self.data_blocks() as usize;
        self.size = 0;
        let mut current_blocks = 0usize;

        // direct
        let direct_clear_count = data_blocks.min(INODE_DIRECT_COUNT);
        while current_blocks < direct_clear_count {
            v.push(self.direct[current_blocks]);
            self.direct[current_blocks] = 0;
            current_blocks += 1;
        }

        // indirect1 block
        if data_blocks > INODE_DIRECT_COUNT {
            v.push(self.indirect1);
            data_blocks -= INODE_DIRECT_COUNT;
            current_blocks = 0;
        } else {
            return v;
        }
        // indirect1
        get_block_cache(self.indirect1 as usize, Arc::clone(block_device))
            .lock()
            .read(0, |indirect1: &IndirectBlock| {
                let indirect1_clear_count = data_blocks.min(INODE_INDIRECT1_COUNT);
                while current_blocks < indirect1_clear_count {
                    v.push(indirect1[current_blocks]);
                    current_blocks += 1;
                }
            });
        self.indirect1 = 0;

        // indirect2 block
        if data_blocks > INODE_INDIRECT1_COUNT {
            v.push(self.indirect2);
            data_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        // indirect2
        assert!(data_blocks <= INODE_INDIRECT2_COUNT);
        let a1 = data_blocks / INODE_INDIRECT1_COUNT;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(self.indirect2 as usize, Arc::clone(block_device))
            .lock()
            .read(0, |indirect2: &IndirectBlock| {
                // full indirect1 blocks
                indirect2.iter().take(a1).for_each(|&entry| {
                    v.push(entry);
                    get_block_cache(entry as usize, Arc::clone(block_device))
                        .lock()
                        .read(0, |indirect1: &IndirectBlock| {
                            v.extend(indirect1.iter());
                        });
                });
                // last indirect1 block
                if b1 > 0 {
                    v.push(indirect2[a1]);
                    get_block_cache(indirect2[a1] as usize, Arc::clone(block_device))
                        .lock()
                        .read(0, |indirect1: &IndirectBlock| {
                            v.extend(indirect1.iter().take(b1));
                        });
                }
            });
        self.indirect2 = 0;
        v
    }

    /// Read data from current disk inode
    pub fn read_at(
        &self,
        offset: usize,
        buf: &mut [u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_SIZE;
        let mut read_size = 0usize;

        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);

            // read and update read size
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = &data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + block_read_size];
                dst.copy_from_slice(src);
            });
            read_size += block_read_size;

            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        read_size
    }

    /// Write data into current disk inode
    /// size must be adjusted properly beforehand
    pub fn write_at(
        &mut self,
        offset: usize,
        buf: &[u8],
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(self.size as usize);
        assert!(start <= end);
        let mut start_block = start / BLOCK_SIZE;
        let mut write_size = 0usize;

        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SIZE + 1) * BLOCK_SIZE;
            end_current_block = end_current_block.min(end);

            // write and update write size
            let block_write_size = end_current_block - start;
            get_block_cache(
                self.get_block_id(start_block as u32, block_device) as usize,
                Arc::clone(block_device),
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let src = &buf[write_size..write_size + block_write_size];
                let dst =
                    &mut data_block[start % BLOCK_SIZE..start % BLOCK_SIZE + block_write_size];
                dst.copy_from_slice(src);
            });
            write_size += block_write_size;

            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        write_size
    }
}

/// A directory entry
#[repr(C)]
pub struct DirEntry {
    name: [u8; NAME_LENGTH_LIMIT + 1],
    inode_number: u32,
}

/// Size of a directory entry
pub const DIRENT_SIZE: usize = core::mem::size_of::<DirEntry>();

impl DirEntry {
    /// Crate a directory entry from name and inode number
    pub fn new(name: &str, inode_number: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_number,
        }
    }

    /// Create an empty directory entry
    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }

    /// Serialize into bytes
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, DIRENT_SIZE) }
    }

    /// Serialize into mutable bytes
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, DIRENT_SIZE) }
    }

    /// Get name of the entry
    pub fn name(&self) -> &str {
        let len = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.name.len());
        core::str::from_utf8(&self.name[..len]).unwrap()
    }

    /// Get inode number of the entry
    pub fn inode_number(&self) -> u32 {
        self.inode_number
    }
}
