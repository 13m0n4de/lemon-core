#![no_std]
#![no_main]

extern crate user_lib;

#[no_mangle]
pub extern "Rust" fn main(argc: usize, argv: &[&str]) -> i32 {
    assert!(argc == argv.len());
    assert!(argv[0] == "cmdline_args");
    assert!(argv[1] == "welcome");
    assert!(argv[2] == "to");
    assert!(argv[3] == "the");
    assert!(argv[4] == "wired");
    assert!(argv[5] == "world");
    0
}
