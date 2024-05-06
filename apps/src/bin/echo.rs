#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

#[no_mangle]
extern "Rust" fn main(_argc: usize, argv: &[&str]) -> i32 {
    let output = argv[1..].join(" ");
    println!("{}", output);
    0
}
