//! # OS Kernel Entrypoint
//!
//! - Includes `entry.asm` for initial setup.
//! - Includes `link_app.S` to link the application with the kernel.
//! - Initializes `.bss` to zero.
//! - Initializes submodules.
//! - call [`task::run_first`] and for the first time go to userspace.
//!
//! Submodules:
//!
//! - [`mm`]: Memory management
//! - [`syscall`]: System call handling and implementation.
//! - [`task`]: Task management
//! - [`trap`]: Handles all cases of switching from userspace to the kernel.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::missing_panics_doc)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
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
global_asm!(include_str!("link_app.S"));

/// the rust entrypoint of OS
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    clear_bss();
    logging::init();
    mm::init();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    task::run_first();
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
