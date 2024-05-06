#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use core::str;
use user_lib::fs::mkdir;

#[no_mangle]
extern "Rust" fn main(argc: usize, argv: &[&str]) -> i32 {
    if argc == 1 {
        println!("missing operand");
        return 1;
    }
    for target in &argv[1..] {
        match mkdir(target) {
            0 => {}
            -1 => println!(
                "cannot create directory '{}': No such file or directory",
                target
            ),
            -2 => println!("cannot create directory '{}': File exists", target),
            _ => panic!(),
        }
    }
    0
}
