#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::ptr::null_mut;

#[no_mangle]
fn main() -> i32 {
    println!("store_fault APP running...");
    println!("Into Test store_fault, we will insert an invalid store operation...");
    println!("Kernel should kill this application!");
    unsafe {
        null_mut::<u8>().write_volatile(1);
    }
    0
}
