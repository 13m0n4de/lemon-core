use alloc::{
    format,
    string::{String, ToString},
    vec,
};
use bitflags::bitflags;

use crate::syscall::{
    sys_chdir, sys_close, sys_dup, sys_dup2, sys_fstat, sys_getcwd, sys_mkdir, sys_open, sys_pipe,
    sys_read, sys_unlink, sys_write,
};

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

/// Gets the current working directory and stores it in the provided string buffer.
///
/// # Panics
///
/// Panics if the current working directory contains invalid UTF-8 sequences.
pub fn getcwd(s: &mut String) -> isize {
    let mut buffer = vec![0u8; 128];
    let len = sys_getcwd(&mut buffer);
    *s = core::str::from_utf8(&buffer[0..len as usize])
        .unwrap()
        .to_string();
    len
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

pub fn dup2(old_fd: usize, new_fd: usize) -> isize {
    sys_dup2(old_fd, new_fd)
}

pub fn mkdir(path: &str) -> isize {
    let path = format!("{path}\0");
    sys_mkdir(&path)
}

pub fn unlink(path: &str, flags: u32) -> isize {
    let path = format!("{path}\0");
    sys_unlink(&path, flags)
}

pub fn chdir(path: &str) -> isize {
    let path = format!("{path}\0");
    sys_chdir(&path)
}

#[allow(clippy::needless_pass_by_value)]
pub fn open(path: &str, flags: OpenFlags) -> isize {
    let path = format!("{path}\0");
    sys_open(&path, flags.bits())
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn fstat(fd: usize, stat: &mut Stat) -> isize {
    sys_fstat(fd, core::ptr::from_mut(stat).cast())
}
