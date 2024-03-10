#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{exec, fork, waitpid};

#[no_mangle]
fn main() -> i32 {
    let mut line: String = String::new();
    print!("~> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                print!("\n");
                if !line.is_empty() {
                    line.push('\0');
                    let pid = fork();
                    if pid == 0 {
                        let args_str: Vec<String> = line
                            .split_whitespace()
                            .map(|arg| format!("{arg}\0"))
                            .collect();
                        let args_addr: Vec<*const u8> =
                            args_str.iter().map(|arg| arg.as_ptr()).collect();

                        if exec(&args_str[0], args_addr.as_slice()) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!("~> ");
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
