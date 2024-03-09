//! File system

mod inode;
mod pipe;
mod stdio;

use crate::mm::UserBuffer;
pub use inode::{list_apps, open_file, OpenFlags};
pub use stdio::{Stdin, Stdout};

/// File trait
pub trait File: Send + Sync {
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;
    /// If readable
    fn is_readable(&self) -> bool;
    /// If writable
    fn is_writable(&self) -> bool;
}
