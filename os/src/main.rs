//! # OS Kernel Entrypoint
//!
//! ## Overview
//!
//! - Include `entry.asm` for initial setup.
//! - Zero out the .bss section.
//! - Initialize heap allocator, frame allocator, and kernel space.
//! - Set CSR `stvec` to the entry point of `__alltraps`.
//! - Enable the timer interrupt and set up the next timer interrupt
//! - Adds the init process to the process manager.
//! - Begins process execution and scheduling.

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[cfg(feature = "board_qemu")]
#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod drivers;
mod fs;
mod lang_items;
mod logging;
mod mm;
mod sbi;
mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

/// the rust entrypoint of OS
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    logging::init();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::add_daemon();
    task::run_tasks();
    panic!("unreachable in rust_main!");
}

/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }

    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}
