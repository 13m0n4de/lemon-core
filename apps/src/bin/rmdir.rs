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
        println!("{}", filename);
        match unlink(filename, AT_REMOVEDIR) {
            0 => {}
            -1 => println!("cannot remove '{}': No such file or directory", filename),
            -2 => println!("failed to remove '{}': Not a directory", filename),
            -3 => println!("failed to remove '{}': Directory not empty", filename),
            _ => panic!(),
        }
    }
    0
}
