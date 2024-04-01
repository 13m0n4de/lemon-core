use bitflags::bitflags;

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 9;
        const TRUNC = 1 << 10;
    }
}

bitflags! {
    #[derive(PartialEq, Eq, Default)]
    pub struct StatMode: u32 {
        const NULL = 0;
        const DIR = 0o040_000;
        const REG = 0o100_000;
        const LNK = 0o120_000;
    }
}

#[repr(C)]
#[derive(Default)]
pub struct Stat {
    pub dev: u32,
    pub ino: u32,
    pub mode: StatMode,
    pub off: usize,
    pub size: u32,
}

impl Stat {
    pub fn new() -> Self {
        Self {
            dev: 0,
            ino: 0,
            mode: StatMode::NULL,
            off: 0,
            size: 0,
        }
    }
}

pub const NAME_LENGTH_LIMIT: usize = 27;

#[repr(C)]
pub struct Dirent {
    pub name: [u8; NAME_LENGTH_LIMIT + 1],
    pub inode_number: u32,
}

pub const DIRENT_SIZE: usize = core::mem::size_of::<Dirent>();

pub const AT_REMOVEDIR: u32 = 1;
