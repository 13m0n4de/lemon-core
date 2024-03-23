#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use user_lib::console::getchar;
use user_lib::fs::*;
use user_lib::process::*;
use user_lib::*;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

struct CommandArguments {
    argc: usize,
    argv: Vec<String>,
    input_file: Option<String>,
    output_file: Option<String>,
}

impl CommandArguments {
    pub fn new(command: &str) -> Self {
        let mut argc = 0;
        let mut argv = Vec::new();
        let mut input_file = None;
        let mut output_file = None;

        let mut args_iter = command.split_whitespace();
        while let Some(arg) = args_iter.next() {
            match arg {
                ">" => {
                    output_file = args_iter.next().map(String::from);
                }
                "<" => {
                    input_file = args_iter.next().map(String::from);
                }
                _ => {
                    argc += 1;
                    argv.push(arg.to_string());
                }
            }
        }

        Self {
            argc,
            argv,
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
        print!("[root@lemon-core:{cwd}]$ ");
        let line = getline();
        if line.is_empty() {
            continue;
        }
        let cmd_args = CommandArguments::new(&line);

        match cmd_args.argv[0].as_str() {
            "cd" => match cmd_args.argc {
                1 => {
                    cd("/");
                    getcwd(&mut cwd);
                }
                2 => {
                    cd(&cmd_args.argv[1]);
                    getcwd(&mut cwd);
                }
                _ => {
                    println!("Too many args for cd command");
                }
            },
            "exit" => break,
            path => {
                if is_dir(path) && cmd_args.argc == 1 {
                    cd(path);
                    getcwd(&mut cwd);
                } else {
                    let pid = fork();
                    if pid == 0 {
                        if let Some(input) = cmd_args.input_file {
                            redirect_io(input, 0, OpenFlags::RDONLY);
                        }
                        if let Some(output) = cmd_args.output_file {
                            redirect_io(output, 1, OpenFlags::CREATE | OpenFlags::WRONLY);
                        }
                        let path = if path.contains('/') {
                            path.to_string()
                        } else {
                            format!("/bin/{path}")
                        };
                        exec(&path, &cmd_args.argv);
                        println!("{}: command not found", path);
                        return -1;
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                    }
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
    assert!(dup2(file_fd, fd) != -1);
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

fn cd(path: &str) {
    match chdir(path) {
        0 => {}
        -1 => println!("{}: No such file or directory", path),
        -2 => println!("{}: Not a directory", path),
        _ => panic!(),
    }
}

fn is_dir(path: &str) -> bool {
    let fd = open(path, OpenFlags::RDONLY);
    if fd == -1 {
        return false;
    }

    let mut stat = Stat::new();
    if fstat(fd as usize, &mut stat) == -1 {
        close(fd as usize);
        return false;
    }
    close(fd as usize);

    matches!(stat.mode, StatMode::DIR)
}
