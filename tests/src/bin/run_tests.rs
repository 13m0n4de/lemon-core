#![no_std]
#![no_main]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_panics_doc)]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::format;
use user_lib::process::{exec, fork, waitpid};

static TESTS: &[(&str, i32)] = &[("huge_write", 0)];

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    let mut num_passed_tests = 0;
    println!("Running {} tests...", TESTS.len());

    for (test_name, expected_exit_code) in TESTS {
        println!("\x1b[4;37m{}\x1b[0m running...", test_name);

        let pid = fork();
        if pid == 0 {
            let test_executable_path = format!("/tests/{test_name}");
            exec(&test_executable_path, &[&test_executable_path]);
            unreachable!();
        }

        let mut exit_code: i32 = Default::default();
        let wait_pid = waitpid(pid as usize, &mut exit_code);
        assert_eq!(pid, wait_pid);

        if exit_code == *expected_exit_code {
            num_passed_tests += 1;
            println!("\x1b[4;37m{}\x1b[0m \x1b[1;32mpassed\x1b[0m", test_name);
        } else {
            println!(
                "\x1b[4;37m{}\x1b[0m \x1b[1;31mfailed (exit code: {})\x1b[0m",
                test_name, exit_code
            );
        }
    }

    println!(
        "{} tests in total, {} succeed, {} failed",
        TESTS.len(),
        num_passed_tests,
        TESTS.len() - num_passed_tests
    );

    0
}
