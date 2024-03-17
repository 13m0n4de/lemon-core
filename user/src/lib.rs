#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
pub mod console;
mod heap_allocator;
mod lang_items;
mod syscall;

pub mod fs;
pub mod signal;

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use fs::*;
use heap_allocator::init_heap;
use syscall::*;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    init_heap();
    let args: Vec<&'static str> = (0..argc)
        .map(|i| {
            let str_start = unsafe {
                ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile()
            };
            let len = (0usize..)
                .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
                .unwrap();
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap()
        })
        .collect();
    exit(main(argc, args.as_slice()))
}

#[no_mangle]
#[linkage = "weak"]
fn main(_argc: usize, _argv: &[&str]) -> i32 {
    panic!("Cannot find main!");
}

pub fn getcwd(s: &mut String) -> isize {
    let mut buffer = vec![0u8; 128];
    let len = sys_getcwd(&mut buffer);
    *s = core::str::from_utf8(&buffer[0..len as usize])
        .unwrap()
        .to_string();
    len
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

pub fn dup2(old_fd: usize, new_fd: usize) -> isize {
    sys_dup2(old_fd, new_fd)
}

pub fn mkdir(path: &str) -> isize {
    let path = format!("{path}\0");
    sys_mkdir(&path)
}

pub fn unlink(path: &str, flags: u32) -> isize {
    let path = format!("{path}\0");
    sys_unlink(&path, flags)
}

pub fn chdir(path: &str) -> isize {
    let path = format!("{path}\0");
    sys_chdir(&path)
}

pub fn open(path: &str, flags: OpenFlags) -> isize {
    let path = format!("{path}\0");
    sys_open(&path, flags.bits())
}

pub fn close(fd: usize) -> isize {
    sys_close(fd)
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize {
    sys_pipe(pipe_fd)
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}

pub fn fstat(fd: usize, stat: &mut Stat) -> isize {
    sys_fstat(fd, stat as *mut _ as *mut _)
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn kill(pid: usize, signum: i32) -> isize {
    sys_kill(pid, signum)
}

pub fn sleep(sleep_ms: usize) {
    sys_sleep(sleep_ms);
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

pub fn thread_create(entry: usize, arg: usize) -> isize {
    sys_thread_create(entry, arg)
}

pub fn gettid() -> isize {
    sys_gettid()
}

pub fn waittid(tid: usize) -> isize {
    loop {
        match sys_waittid(tid) {
            -2 => {
                yield_();
            }
            exit_code => return exit_code,
        }
    }
}
