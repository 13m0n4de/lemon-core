/// Use a block size of 512 bytes
pub const BLOCK_SIZE: usize = 512;
/// Number of bits in a block
pub const BLOCK_BITS: usize = BLOCK_SIZE * 8;
/// Use a block cache of 16 blocks
pub const BLOCK_CACHE_SIZE: usize = 16;

/// Magic number for sanity check
pub const EFS_MAGIC: u32 = 0x3b800001;

/// The max number of direct inodes
pub const INODE_DIRECT_COUNT: usize = 28;
/// The upper bound of direct inode index
pub const DIRECT_BOUND: usize = INODE_DIRECT_COUNT;

/// The max number of indirect1 inodes
pub const INODE_INDIRECT1_COUNT: usize = BLOCK_SIZE / 4;
/// The upper bound of indirect1 inode index
pub const INDIRECT1_BOUND: usize = DIRECT_BOUND + INODE_INDIRECT1_COUNT;
