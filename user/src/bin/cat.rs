#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{close, open, read, OpenFlags};

#[no_mangle]
pub fn main(_argc: usize, argv: &[&str]) -> i32 {
    for filename in &argv[1..] {
        let fd = open(filename, OpenFlags::RDONLY);
        if fd == -1 {
            println!("{}: No such file or directory", filename);
            continue;
        }

        let mut buf = [0u8; 256];
        let fd = fd as usize;

        loop {
            let size = read(fd, &mut buf) as usize;
            if size == 0 {
                break;
            }
            print!("{}", core::str::from_utf8(&buf[..size]).unwrap());
        }

        close(fd);
    }
    0
}