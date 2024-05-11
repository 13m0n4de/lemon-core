/// Use a block size of 512 bytes
pub const BLOCK_SIZE: usize = 512;
/// Number of bits in a block
pub const BLOCK_BITS: usize = BLOCK_SIZE * 8;
/// Use a block cache of 16 blocks
pub const BLOCK_CACHE_SIZE: usize = 16;

/// Magic number for sanity check
pub const EFS_MAGIC: u32 = 0x3b80_0001;

/// The max number of direct inodes
pub const DIRECT_COUNT: usize = 27;
/// The number of indirect inodes
pub const INDIRECT_COUNT: usize = BLOCK_SIZE / 4;
/// The max number of indirect1 inodes
pub const INDIRECT1_COUNT: usize = INDIRECT_COUNT;
/// The max number of indirect2 inodes
pub const INDIRECT2_COUNT: usize = INDIRECT_COUNT.pow(2);
/// The max number of indirect3 inodes
pub const INDIRECT3_COUNT: usize = INDIRECT_COUNT.pow(3);

/// The upper bound of direct inode index
pub const DIRECT_BOUND: usize = DIRECT_COUNT;
/// The upper bound of indirect1 inode index
pub const INDIRECT1_BOUND: usize = DIRECT_BOUND + INDIRECT1_COUNT;
/// The upper bound of indirect2 inode indexs
pub const INDIRECT2_BOUND: usize = INDIRECT1_BOUND + INDIRECT2_COUNT;
#[allow(unused)]
/// The upper bound of indirect3 inode indexs
pub const INDIRECT3_BOUND: usize = INDIRECT2_BOUND + INDIRECT3_COUNT;

/// The max length of inode name
pub const NAME_LENGTH_LIMIT: usize = 27;
