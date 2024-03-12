pub const NAME_LENGTH_LIMIT: usize = 27;

pub const CHR: usize = 0;
pub const REG: usize = 1;
pub const DIR: usize = 2;

#[derive(Default)]
pub struct Stat {
    pub ino: u32,
    pub mode: u32,
    pub off: u32,
    pub size: u32,
}

#[repr(C)]
pub struct Dirent {
    pub name: [u8; NAME_LENGTH_LIMIT + 1],
    pub inode_number: u32,
}

pub const DIRENT_SIZE: usize = core::mem::size_of::<Dirent>();
