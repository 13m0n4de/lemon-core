#![no_std]
#![no_main]

extern crate alloc;
extern crate user_lib;

use alloc::vec;
use user_lib::fs::*;
use user_lib::*;

#[no_mangle]
fn main(argc: usize, argv: &[&str]) -> i32 {
    let targets = if argc > 1 { &argv[1..] } else { &[".\0"] };
    for target in targets {
        list(target);
    }
    0
}

fn list(target: &str) {
    let fd = open(target, OpenFlags::RDONLY);
    let mut stat = Stat::new();
    if fd == -1 {
        println!("cannot access '{}': No such file or directory", target);
        return;
    }
    match fstat(fd as usize, &mut stat) {
        0 => {}
        -1 => {
            println!("{}: Bad file descriptor", fd);
            return;
        }
        _ => panic!(),
    }

    match stat.mode {
        StatMode::REG => {
            println!("{}", target);
        }
        StatMode::DIR => {
            let size = stat.size as usize;
            let mut buf = vec![0u8; size];
            read(fd as usize, &mut buf);
            for i in 2..size / DIRENT_SIZE {
                let offset = i * DIRENT_SIZE;
                let dirent = unsafe { &*(buf.as_ptr().add(offset) as *const Dirent) };
                let len = dirent
                    .name
                    .iter()
                    .position(|&v| v == 0)
                    .unwrap_or(dirent.name.len());
                let name = core::str::from_utf8(&dirent.name[..len]).unwrap();
                print!("{}\n", name);
            }
        }
        _ => panic!("Unknown mode"),
    }
    close(fd as usize);
}
