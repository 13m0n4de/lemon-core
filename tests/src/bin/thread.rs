#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{
    process::exit,
    thread::{thread_create, waittid},
};

pub fn thread_a() -> ! {
    for _ in 0..1000 {
        print!("a");
    }
    exit(1)
}

pub fn thread_b() -> ! {
    for _ in 0..1000 {
        print!("b");
    }
    exit(2)
}

pub fn thread_c() -> ! {
    for _ in 0..1000 {
        print!("c");
    }
    exit(3)
}

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    let v = &[
        thread_create(thread_a as usize, 0),
        thread_create(thread_b as usize, 0),
        thread_create(thread_c as usize, 0),
    ];
    for tid in v {
        let exit_code = waittid(*tid as usize);
        println!("thread#{} exited with code {}", tid, exit_code);
    }
    println!("main thread exited.");
    0
}
