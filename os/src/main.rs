//! The main module and entrypoint

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]
#![no_main]

#[macro_use]
mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;

global_asm!(include_str!("entry.asm"));

/// the rust entrypoint of OS
#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello world!");
    loop {}
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
