#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec;
use user_lib::fs::{OpenFlags, Stat, StatMode, close, fstat, open, read};

#[no_mangle]
extern "Rust" fn main(_argc: usize, argv: &[&str]) -> i32 {
    for filename in &argv[1..] {
        let fd = open(filename, OpenFlags::RDONLY);
        if fd == -1 {
            println!("{}: No such file or directory", filename);
            continue;
        }

        let mut stat = Stat::new();
        match fstat(fd as usize, &mut stat) {
            0 => {}
            -1 => {
                println!("{}: Bad file descriptor", fd);
                continue;
            }
            _ => panic!(),
        }

        let fd = fd as usize;
        let size = stat.size as usize;

        match stat.mode {
            StatMode::REG => {
                let mut buf = vec![0u8; size];
                read(fd as usize, &mut buf);
                print!("{}", core::str::from_utf8(&buf[..size]).unwrap());
            }
            _ => println!("{}: Is not a file", filename),
        }

        close(fd);
    }
    0
}
