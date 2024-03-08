#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::ptr::{read_volatile, NonNull};

#[no_mangle]
fn main() -> i32 {
    println!("load_fault APP running...");
    println!("Into Test load_fault, we will insert an invalid load operation...");
    println!("Kernel should kill this application!");
    unsafe {
        let _i = read_volatile(NonNull::<u8>::dangling().as_ptr());
    }
    0
}
