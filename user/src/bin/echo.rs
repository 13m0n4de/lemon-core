#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use user_lib::*;

#[no_mangle]
pub fn main(_argc: usize, argv: &[&str]) -> i32 {
    let output = argv[1..].join(" ");
    println!("{}", output);
    0
}
