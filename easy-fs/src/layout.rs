use alloc::{sync::Arc, vec::Vec};

use crate::{
    block_cache,
    block_dev::BlockDevice,
    config::{
        BLOCK_SIZE, DIRECT_BOUND, DIRECT_COUNT, EFS_MAGIC, INDIRECT1_BOUND, INDIRECT1_COUNT,
        INDIRECT2_BOUND, INDIRECT2_COUNT, INDIRECT_COUNT, NAME_LENGTH_LIMIT,
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
    #[inline]
    pub fn init(
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
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.magic == EFS_MAGIC
    }
}

/// Type of a disk inode
#[derive(PartialEq)]
pub enum DiskInodeKind {
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
    kind: DiskInodeKind,
    pub size: u32,
    pub direct: [u32; DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
    pub indirect3: u32,
}

impl DiskInode {
    /// Initialize a disk inode
    #[inline]
    pub fn init(&mut self, kind: DiskInodeKind) {
        self.kind = kind;
        self.size = 0;
        self.direct.fill(0);
        self.indirect1 = 0;
        self.indirect2 = 0;
    }

    /// Whether this inode is a directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.kind == DiskInodeKind::Directory
    }

    /// Whether this inode is a file
    #[allow(unused)]
    #[inline]
    pub fn is_file(&self) -> bool {
        self.kind == DiskInodeKind::File
    }

    /// Get id of block given inner id
    pub fn block_id(&self, block_index: u32, block_device: &Arc<dyn BlockDevice>) -> u32 {
        let block_index = block_index as usize;
        if block_index < DIRECT_BOUND {
            self.direct[block_index]
        } else if block_index < INDIRECT1_BOUND {
            block_cache::get(self.indirect1 as usize, block_device)
                .lock()
                .read(0, |indirect_block: &IndirectBlock| {
                    indirect_block[block_index - DIRECT_BOUND]
                })
        } else if block_index < INDIRECT2_BOUND {
            let index = block_index - INDIRECT1_BOUND;
            let indirect1 = block_cache::get(self.indirect2 as usize, block_device)
                .lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[index / INDIRECT1_COUNT]
                });
            block_cache::get(indirect1 as usize, block_device)
                .lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[index % INDIRECT1_COUNT]
                })
        } else {
            let index = block_index - INDIRECT2_BOUND;
            let indirect2 = block_cache::get(self.indirect3 as usize, block_device)
                .lock()
                .read(0, |indirect3: &IndirectBlock| {
                    indirect3[index / INDIRECT2_COUNT]
                });
            let indirect1 = block_cache::get(indirect2 as usize, block_device)
                .lock()
                .read(0, |indirect2: &IndirectBlock| {
                    indirect2[index % INDIRECT2_COUNT / INDIRECT1_COUNT]
                });
            block_cache::get(indirect1 as usize, block_device)
                .lock()
                .read(0, |indirect1: &IndirectBlock| {
                    indirect1[index % INDIRECT2_COUNT % INDIRECT1_COUNT]
                })
        }
    }

    #[inline]
    fn count_data_block(size: u32) -> u32 {
        size.div_ceil(BLOCK_SIZE as u32)
    }

    /// Return number of blocks needed include indirect1/2.
    pub fn count_total_block(size: u32) -> u32 {
        let data_blocks = Self::count_data_block(size) as usize;
        let mut total = Self::count_data_block(size) as usize;
        // indirect1
        if data_blocks > DIRECT_BOUND {
            total += 1;
        }
        // indirect2
        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            let remaining = data_blocks - INDIRECT1_BOUND;
            let indirect1_needed = remaining.div_ceil(INDIRECT1_COUNT);
            total += indirect1_needed.min(INDIRECT1_COUNT);
        }

        // indirect3
        if data_blocks > INDIRECT2_BOUND {
            total += 1;
            let remaining = data_blocks - INDIRECT2_BOUND;
            let indirect2_needed = remaining.div_ceil(INDIRECT2_COUNT);
            let indirect3_needed = remaining.div_ceil(INDIRECT1_COUNT);
            total += indirect2_needed + indirect3_needed;
        }

        total as u32
    }

    /// Get the number of data blocks that have to be allocated given the new size of data
    pub fn blocks_num_needed(&self, new_size: u32) -> u32 {
        assert!(new_size >= self.size);
        Self::count_total_block(new_size) - Self::count_total_block(self.size)
    }

    /// Increase the size of current disk inode
    pub fn increase_size(
        &mut self,
        new_size: u32,
        new_blocks: Vec<u32>,
        block_device: &Arc<dyn BlockDevice>,
    ) {
        let mut block_index = Self::count_data_block(self.size) as usize;
        self.size = new_size;
        let mut new_total_blocks = Self::count_data_block(self.size) as usize;
        let mut new_blocks = new_blocks.into_iter();

        // -------------------- Direct Blocks --------------------
        while block_index < new_total_blocks.min(DIRECT_COUNT) {
            self.direct[block_index] = new_blocks.next().unwrap();
            block_index += 1;
        }
        // ----------------- End of Direct Blocks ----------------

        if new_total_blocks <= DIRECT_COUNT {
            return;
        }

        // -------------------- Indirect Level 1 -----------------
        if block_index == DIRECT_COUNT {
            self.indirect1 = new_blocks.next().unwrap();
        }
        block_index -= DIRECT_COUNT;
        new_total_blocks -= DIRECT_COUNT;

        let cur_leaf = Self::build_tree(
            &mut new_blocks,
            self.indirect1,
            0,
            block_index,
            new_total_blocks,
            0,
            1,
            block_device,
        );
        block_index = block_index.max(cur_leaf);
        // ----------------- End of Indirect Level 1 ------------

        if new_total_blocks <= INDIRECT1_COUNT {
            return;
        }

        // -------------------- Indirect Level 2 -----------------
        if block_index == INDIRECT1_COUNT {
            self.indirect2 = new_blocks.next().unwrap();
        }
        block_index -= INDIRECT1_COUNT;
        new_total_blocks -= INDIRECT1_COUNT;

        let cur_leaf = Self::build_tree(
            &mut new_blocks,
            self.indirect2,
            0,
            block_index,
            new_total_blocks,
            0,
            2,
            block_device,
        );
        block_index = block_index.max(cur_leaf);
        // ----------------- End of Indirect Level 2 ------------

        if new_total_blocks <= INDIRECT2_COUNT {
            return;
        }

        // -------------------- Indirect Level 3 -----------------
        if block_index == INDIRECT2_COUNT {
            self.indirect3 = new_blocks.next().unwrap();
        }
        block_index -= INDIRECT2_COUNT;
        new_total_blocks -= INDIRECT2_COUNT;

        Self::build_tree(
            &mut new_blocks,
            self.indirect3,
            0,
            block_index,
            new_total_blocks,
            0,
            3,
            block_device,
        );
        // ----------------- End of Indirect Level 3 ------------
    }

    #[allow(clippy::too_many_arguments)]
    fn build_tree(
        blocks: &mut alloc::vec::IntoIter<u32>,
        block_id: u32,
        mut cur_leaf: usize,
        src_leaf: usize,
        dst_leaf: usize,
        cur_depth: usize,
        dst_depth: usize,
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        if cur_depth == dst_depth {
            return cur_leaf + 1;
        }
        block_cache::get(block_id as usize, block_device)
            .lock()
            .modify(0, |indirect_block: &mut IndirectBlock| {
                for indirect in indirect_block.iter_mut().take(INDIRECT_COUNT) {
                    if cur_leaf >= dst_leaf {
                        break;
                    }
                    if cur_leaf >= src_leaf {
                        *indirect = blocks.next().unwrap();
                    }
                    cur_leaf = Self::build_tree(
                        blocks,
                        *indirect,
                        cur_leaf,
                        src_leaf,
                        dst_leaf,
                        cur_depth + 1,
                        dst_depth,
                        block_device,
                    );
                }
            });
        cur_leaf
    }

    fn collect_tree(
        collected: &mut Vec<u32>,
        block_id: u32,
        mut cur_leaf: usize,
        max_leaf: usize,
        cur_depth: usize,
        dst_depth: usize,
        block_device: &Arc<dyn BlockDevice>,
    ) -> usize {
        if cur_depth == dst_depth {
            return cur_leaf + 1;
        }
        block_cache::get(block_id as usize, block_device)
            .lock()
            .read(0, |indirect_block: &IndirectBlock| {
                for indirect in indirect_block.iter().take(INDIRECT_COUNT) {
                    if cur_leaf >= max_leaf {
                        break;
                    }
                    collected.push(*indirect);
                    cur_leaf = Self::collect_tree(
                        collected,
                        *indirect,
                        cur_leaf,
                        max_leaf,
                        cur_depth + 1,
                        dst_depth,
                        block_device,
                    );
                }
            });
        cur_leaf
    }

    /// Decrease the size
    pub fn decrease_size(
        &mut self,
        new_size: u32,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Vec<u32> {
        let mut drop_data_blocks: Vec<u32> = Vec::new();
        let mut block_index = Self::count_data_block(self.size) as usize;
        self.size = new_size;
        let mut recycled_blocks = Self::count_data_block(self.size) as usize;

        // -------------------- Direct Blocks --------------------
        let direct_recycle_count = block_index.min(DIRECT_COUNT);
        while recycled_blocks < direct_recycle_count {
            drop_data_blocks.push(self.direct[recycled_blocks]);
            self.direct[recycled_blocks] = 0;
            recycled_blocks += 1;
        }
        // ----------------- End of Direct Blocks ----------------

        if recycled_blocks <= DIRECT_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 1 -----------------
        if block_index == DIRECT_COUNT {
            drop_data_blocks.push(self.indirect1);
            self.indirect1 = 0;
        }
        block_index -= DIRECT_COUNT;
        recycled_blocks -= DIRECT_COUNT;

        let cur_leaf = Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect1,
            0,
            recycled_blocks,
            0,
            1,
            block_device,
        );
        block_index = block_index.min(cur_leaf);
        // ----------------- End of Indirect Level 1 ------------

        if recycled_blocks <= INDIRECT1_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 2 -----------------
        if block_index == INDIRECT1_COUNT {
            drop_data_blocks.push(self.indirect2);
            self.indirect2 = 0;
        }
        block_index -= INDIRECT1_COUNT;
        recycled_blocks -= INDIRECT1_COUNT;

        let cur_leaf = Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect2,
            0,
            recycled_blocks,
            0,
            2,
            block_device,
        );
        block_index = block_index.min(cur_leaf);
        // ----------------- End of Indirect Level 2 ------------

        if recycled_blocks <= INDIRECT2_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 3 -----------------
        if block_index == INDIRECT2_COUNT {
            drop_data_blocks.push(self.indirect3);
            self.indirect3 = 0;
        }
        // block_index -= INDIRECT1_COUNT;
        recycled_blocks -= INDIRECT1_COUNT;

        Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect3,
            0,
            recycled_blocks,
            0,
            3,
            block_device,
        );
        // ----------------- End of Indirect Level 3 ------------

        drop_data_blocks
    }

    /// Clear size to zero and return blocks that should be deallocated.
    /// We will clear the block contents to zero later.
    pub fn clear_size(&mut self, block_device: &Arc<dyn BlockDevice>) -> Vec<u32> {
        let mut drop_data_blocks: Vec<u32> = Vec::new();
        let mut data_blocks = Self::count_data_block(self.size) as usize;
        self.size = 0;

        // -------------------- Direct Blocks --------------------
        drop_data_blocks.extend_from_slice(&self.direct[..data_blocks.min(DIRECT_COUNT)]);
        self.direct.fill(0);
        // ----------------- End of Direct Blocks ----------------

        if data_blocks <= DIRECT_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 1 -----------------
        drop_data_blocks.push(self.indirect1);
        data_blocks -= DIRECT_COUNT;

        Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect1,
            0,
            data_blocks,
            0,
            1,
            block_device,
        );

        self.indirect1 = 0;
        // ----------------- End of Indirect Level 1 ------------

        if data_blocks <= INDIRECT1_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 2 -----------------
        drop_data_blocks.push(self.indirect2);
        data_blocks -= INDIRECT1_COUNT;

        Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect2,
            0,
            data_blocks,
            0,
            2,
            block_device,
        );

        self.indirect2 = 0;
        // ----------------- End of Indirect Level 2 ------------

        if data_blocks <= INDIRECT2_COUNT {
            return drop_data_blocks;
        }

        // -------------------- Indirect Level 3 -----------------
        drop_data_blocks.push(self.indirect3);
        data_blocks -= INDIRECT2_COUNT;

        Self::collect_tree(
            &mut drop_data_blocks,
            self.indirect3,
            0,
            data_blocks,
            0,
            3,
            block_device,
        );

        self.indirect3 = 0;
        // ----------------- End of Indirect Level 2 ------------

        drop_data_blocks
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
            block_cache::get(
                self.block_id(start_block as u32, block_device) as usize,
                block_device,
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
            block_cache::get(
                self.block_id(start_block as u32, block_device) as usize,
                block_device,
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
    #[inline]
    pub fn new(name: &str, inode_number: u32) -> Self {
        let mut bytes = [0u8; NAME_LENGTH_LIMIT + 1];
        bytes[..name.len()].copy_from_slice(name.as_bytes());
        Self {
            name: bytes,
            inode_number,
        }
    }

    /// Create an empty directory entry
    #[inline]
    pub fn empty() -> Self {
        Self {
            name: [0u8; NAME_LENGTH_LIMIT + 1],
            inode_number: 0,
        }
    }

    /// Serialize into bytes
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(core::ptr::from_ref(self).cast::<u8>(), DIRENT_SIZE) }
    }

    /// Serialize into mutable bytes
    #[inline]
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut(core::ptr::from_mut(self).cast::<u8>(), DIRENT_SIZE)
        }
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
    #[inline]
    pub fn inode_number(&self) -> u32 {
        self.inode_number
    }
}
