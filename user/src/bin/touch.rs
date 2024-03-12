#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::fs::*;
use user_lib::*;

#[no_mangle]
pub fn main(argc: usize, argv: &[&str]) -> i32 {
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
