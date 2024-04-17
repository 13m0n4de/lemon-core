#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

#[macro_use]
extern crate user_lib;

#[allow(unconditional_recursion)]
fn f(depth: usize) {
    if depth % 10 == 0 {
        println!("depth = {}", depth);
    }
    f(depth + 1);
}

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    println!("It should trigger segmentation fault!");
    f(0);
    0
}
