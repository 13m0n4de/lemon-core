#![no_std]
#![no_main]

extern crate alloc;

extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::*;

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

        let mut args_ptrs: Vec<*const u8> = args_str.iter().map(|arg| arg.as_ptr()).collect();
        args_ptrs.push(core::ptr::null());

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
    let mut cwd = String::new();
    getcwd(&mut cwd);
    loop {
        print!("{cwd} ~> ");
        let line = getline();
        if line.is_empty() {
            continue;
        }
        let cmd_args = CommandArguments::new(&line);

        match cmd_args.args_str[0].as_str() {
            "cd\0" => cd(&mut cwd, &cmd_args.args_str),
            "exit\0" => break,
            _ => {
                let pid = fork();
                if pid == 0 {
                    if let Some(input) = cmd_args.input_file {
                        redirect_io(input, 0, OpenFlags::RDONLY);
                    }
                    if let Some(output) = cmd_args.output_file {
                        redirect_io(output, 1, OpenFlags::CREATE | OpenFlags::WRONLY);
                    }
                    exec(
                        &format!("/bin/{}", &cmd_args.args_str[0]),
                        cmd_args.args_ptrs.as_slice(),
                    );
                    println!("{}: command not found", cmd_args.args_str[0]);
                } else {
                    let mut exit_code: i32 = 0;
                    let exit_pid = waitpid(pid as usize, &mut exit_code);
                    assert_eq!(pid, exit_pid);
                }
            }
        }
    }
    0
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

fn getline() -> String {
    let mut input = String::new();
    loop {
        match getchar() {
            DL => {
                if !input.is_empty() {
                    input.pop();
                    print!("{} {}", BS as char, BS as char);
                }
            }
            LF | CR => {
                print!("\n");
                break input;
            }
            ch => {
                print!("{}", ch as char);
                input.push(ch as char)
            }
        }
    }
}

fn cd(cwd: &mut String, args: &[String]) {
    let path = match args.len() {
        1 => String::from("/"),
        2 => String::from(&args[1]),
        _ => {
            println!("Too many args for cd command");
            return;
        }
    };

    match chdir(&path) {
        0 => {}
        -1 => println!("{}: No such file or directory", args[1]),
        -2 => println!("{}: Not a directory", args[1]),
        _ => panic!(),
    }
    getcwd(cwd);
}
