use alloc::{format, string::String, vec::Vec};

use crate::syscall::*;

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn getpid() -> isize {
    sys_getpid()
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec<T: AsRef<str>>(path: &str, args: &[T]) -> isize {
    let path = format!("{path}\0");
    let args: Vec<String> = args
        .iter()
        .map(|arg| format!("{}\0", arg.as_ref()))
        .collect();
    let mut arg_ptrs: Vec<*const u8> = args.iter().map(|s| s.as_ptr()).collect();
    arg_ptrs.push(core::ptr::null());
    sys_exec(&path, &arg_ptrs)
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}
