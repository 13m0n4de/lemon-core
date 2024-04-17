#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

#[macro_use]
extern crate user_lib;

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe {
        core::ptr::null_mut::<u8>().write_volatile(0);
    }
    0
}
