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
use user_lib::{close, dup, exec, fork, open, waitpid, OpenFlags};

struct CommandArguments {
    args_str: Vec<String>,
    args_ptrs: Vec<*const u8>,
    input_file: Option<String>,
    output_file: Option<String>,
}

impl CommandArguments {
    pub fn new(command: &str) -> Self {
        let mut args_str = Vec::new();
        let mut input_file = None;
        let mut output_file = None;

        let mut args_iter = command.split_whitespace().map(|arg| format!("{arg}\0"));
        while let Some(arg) = args_iter.next() {
            match arg.as_str() {
                ">\0" => {
                    output_file = args_iter.next();
                }
                "<\0" => {
                    input_file = args_iter.next();
                }
                _ => args_str.push(arg),
            }
        }

        let args_ptrs: Vec<*const u8> = args_str.iter().map(|arg| arg.as_ptr()).collect();

        Self {
            args_str,
            args_ptrs,
            input_file,
            output_file,
        }
    }
}

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
                    let pid = fork();
                    if pid == 0 {
                        let cmd_args = CommandArguments::new(&line.clone());
                        if let Some(input) = cmd_args.input_file {
                            redirect_io(input, 0, OpenFlags::RDONLY);
                        }
                        if let Some(output) = cmd_args.output_file {
                            redirect_io(output, 1, OpenFlags::CREATE | OpenFlags::WRONLY);
                        }
                        exec(&cmd_args.args_str[0], cmd_args.args_ptrs.as_slice());
                        println!("{}: command not found", cmd_args.args_str[0]);
                        return -1;
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

fn redirect_io(file_name: String, fd: usize, flags: OpenFlags) {
    let file_fd = open(&file_name, flags);
    if file_fd == -1 {
        println!("Error when opening file {}", file_name);
        return;
    }
    let file_fd = file_fd as usize;
    close(fd);
    assert_eq!(dup(file_fd), fd as isize);
    close(file_fd);
}
