#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
    fs::{close, open, write, OpenFlags},
    process::get_time,
};

const BUFFER_SIZE: usize = 1024; // 1KiB
const SIZE_MB: usize = 1; // Size in MiB to write

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    let buffer: [u8; BUFFER_SIZE] = core::array::from_fn(|i| i as u8);

    let fd = open("test_file", OpenFlags::CREATE | OpenFlags::WRONLY);
    assert!(fd >= 0, "Open test file failed!");

    let fd = fd as usize;
    let start = get_time();
    let mut total_written = 0usize;

    for _ in 0..SIZE_MB << 10 {
        let written = write(fd, &buffer);
        total_written += written as usize;
    }

    close(fd);

    let time_ms = (get_time() - start) as usize;
    println!(
        "{}MiB written, time cost = {}ms, write speed = {}KiB/s",
        total_written >> 20,
        time_ms,
        (total_written >> 10) / (time_ms / 1000)
    );

    0
}
