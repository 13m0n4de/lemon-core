#![no_std]
#![no_main]

extern crate user_lib;

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    #[allow(clippy::empty_loop)]
    loop {}
}
