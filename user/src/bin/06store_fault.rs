#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::ptr::NonNull;

#[no_mangle]
fn main() -> i32 {
    println!("store_fault APP running...");
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe {
        NonNull::<u8>::dangling().as_ptr().write_volatile(1);
    }
    0
}
