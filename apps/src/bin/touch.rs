#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use user_lib::fs::{close, open, OpenFlags};

#[no_mangle]
extern "Rust" fn main(argc: usize, argv: &[&str]) -> i32 {
    if argc == 1 {
        println!("missing operand");
        return 1;
    }
    for filename in &argv[1..] {
        let fd = open(filename, OpenFlags::RDONLY | OpenFlags::CREATE);
        if fd == -1 {
            println!("cannot access '{}'", filename);
            continue;
        }
        close(fd as usize);
    }
    0
}
