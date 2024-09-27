#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use alloc::vec;
use user_lib::fs::{close, open, read, write, OpenFlags};

static STR: &str = "Hello, world!";

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    let fd = open("test_file", OpenFlags::CREATE | OpenFlags::WRONLY);
    assert!(fd >= 0, "Open test file failed!");

    let fd = fd as usize;
    assert_eq!(write(fd, STR.as_bytes()), STR.len() as isize);

    close(fd);

    let fd = open("test_file", OpenFlags::RDONLY);
    assert!(fd >= 0, "Re-open file for reading failed!");

    let fd = fd as usize;
    let mut buf = vec![0u8; STR.len()];
    assert_eq!(read(fd as usize, &mut buf), STR.len() as isize);
    assert_eq!(&buf, STR.as_bytes());

    close(fd);
    assert!(read(fd, &mut buf) == -1);

    0
}
