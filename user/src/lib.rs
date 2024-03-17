#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

mod heap_allocator;
mod lang_items;

#[macro_use]
pub mod console;
pub mod fs;
pub mod process;
pub mod signal;
pub mod sync;
pub mod syscall;
pub mod thread;

use alloc::vec::Vec;
use heap_allocator::init_heap;
use process::exit;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    init_heap();
    let args: Vec<&'static str> = (0..argc)
        .map(|i| {
            let str_start = unsafe {
                ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile()
            };
            let len = (0usize..)
                .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
                .unwrap();
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap()
        })
        .collect();
    exit(main(argc, args.as_slice()))
}

#[no_mangle]
#[linkage = "weak"]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}
