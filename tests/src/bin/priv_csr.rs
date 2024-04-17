#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]

#[macro_use]
extern crate user_lib;

use riscv::register::sstatus::{self, SPP};

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    println!("Try to access privileged CSR in U Mode");
    println!("Kernel should kill this application!");
    unsafe {
        sstatus::set_spp(SPP::User);
    }
    0
}
