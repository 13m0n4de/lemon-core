//! # OS Kernel Entrypoint
//!
//! - Includes `entry.asm` for initial setup.
//! - Includes `link_app.S` to link the application with the kernel.
//! - Initializes `.bss` to zero.
//! - Initializes logging.
//! - Displays memory segment layouts (`.text`, `.rodata`, `.data`, `.bss`).
//! - Initializes trap.
//! - call [`batch::run_next_app`] and for the first time go to userspace.
//!
//! Submodules:
//!
//! - [`batch`]: Manages the loading and execution of multiple applications.
//! - [`trap`]: Handles all cases of switching from userspace to the kernel.
//! - [`syscall`]: System call handling and implementation.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;
mod batch;
mod lang_items;
mod logging;
mod sbi;
mod sync;
mod syscall;
mod trap;

use core::arch::global_asm;
use log::debug;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

/// the rust entrypoint of OS
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    /* External Symbols:
     * - `stext`: start addr of text segment
     * - `etext`: end addr of text segment
     * - `srodata`: start addr of Read-Only data segment
     * - `erodata`: end addr of Read-Only data segment
     * - `sdata`: start addr of data segment
     * - `edata`: end addr of data segment
     * - `sbss`: start addr of BSS segment
     * - `ebss`: end addr of BSS segment
     * - `boot_stack_lower_bound`: lower bound of the boot stack
     * - `boot_stack_top`: top addr of the boot stack
     */
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack_lower_bound();
        fn boot_stack_top();
    }

    clear_bss();
    logging::init();

    debug!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize, etext as usize
    );
    debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    debug!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    debug!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
    debug!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);

    trap::init();
    batch::print_app_info();
    batch::run_next_app();
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
