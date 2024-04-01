//! # OS Kernel Entrypoint
//!
//! - Includes `entry.asm` for initial setup.
//! - Initializes `.bss` to zero.
//! - Initializes logging.
//! - Displays memory segment layouts (`.text`, `.rodata`, `.data`, `.bss`).
//! - Outputs "Hello world!" then shuts down.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

use core::arch::global_asm;
use log::{debug, info};

global_asm!(include_str!("entry.asm"));

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

    info!("[kernel] Hello world!");

    sbi::shutdown(false)
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
