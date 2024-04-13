#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]

extern crate alloc;

mod heap_allocator;
mod lang_items;

#[macro_use]
pub mod console;
pub mod fs;
pub mod gui;
pub mod input;
pub mod process;
pub mod signal;
pub mod sync;
pub mod syscall;
pub mod thread;

use alloc::vec::Vec;
use heap_allocator::init_heap;
use process::exit;

/// Entry point for the application.
///
/// # Panics
///
/// Panics if it fails to find a null terminator indicating the end of a C-style string.
#[no_mangle]
#[link_section = ".text.entry"]
#[allow(clippy::similar_names)]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    init_heap();
    let args: Vec<&'static str> = (0..argc)
        .map(|i| {
            let str_start = unsafe {
                ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile()
            };
            let len = (0usize..usize::MAX)
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
extern "Rust" fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}
