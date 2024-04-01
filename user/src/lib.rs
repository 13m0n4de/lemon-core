#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    exit(main())
}

#[no_mangle]
#[linkage = "weak"]
pub extern "Rust" fn main() -> i32 {
    panic!("Cannot find main!");
}

use syscall::{sys_exit, sys_get_time, sys_write, sys_yield};

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn get_time() -> isize {
    sys_get_time()
}
