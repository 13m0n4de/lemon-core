#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::process::{exit, fork, wait, waitpid, yield_};

const EXIT_MAGIC: i32 = -0x10384;

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    println!("Parent: starting child process.");

    let pid = fork();
    if pid == 0 {
        println!("Child: running.");
        for _ in 0..7 {
            yield_();
        }
        exit(EXIT_MAGIC);
    } else {
        println!("Parent: created child with PID: {}", pid);
    }

    println!("Parent: waiting for child to exit.");
    let mut exit_code: i32 = Default::default();
    assert!(waitpid(pid as usize, &mut exit_code) == pid && exit_code == EXIT_MAGIC);
    assert!(waitpid(pid as usize, &mut exit_code) < 0 && wait(&mut exit_code) <= 0);
    println!("Parent: waited on child with PID: {}", pid);

    0
}
