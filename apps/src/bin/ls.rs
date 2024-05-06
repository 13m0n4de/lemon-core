#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate user_lib;

use alloc::vec;
use user_lib::fs::{close, fstat, open, read, Dirent, OpenFlags, Stat, StatMode, DIRENT_SIZE};

#[no_mangle]
extern "Rust" fn main(argc: usize, argv: &[&str]) -> i32 {
    let targets = if argc > 1 { &argv[1..] } else { &["."] };
    for target in targets {
        list(target);
    }
    0
}

fn list(target: &str) {
    let fd = open(target, OpenFlags::RDONLY);
    if fd == -1 {
        println!("cannot access '{}': No such file or directory", target);
        return;
    }

    let mut stat = Stat::new();
    match fstat(fd as usize, &mut stat) {
        0 => {}
        -1 => {
            println!("{}: Bad file descriptor", fd);
            return;
        }
        _ => panic!("Unexpected fstat error"),
    }

    match stat.mode {
        StatMode::REG => {
            println!("{}", target);
        }
        StatMode::DIR => {
            let size = stat.size as usize;
            let mut buf = vec![0u8; size];
            read(fd as usize, &mut buf);
            let entries = buf.chunks_exact(DIRENT_SIZE);
            for entry in entries.skip(2) {
                let dirent: &Dirent = unsafe { &entry.as_ptr().cast::<Dirent>().read_unaligned() };
                let name_len = dirent.name.iter().take_while(|&&c| c != 0).count();
                let name = core::str::from_utf8(&dirent.name[..name_len])
                    .expect("Invalid UTF-8 in directory name");
                print!("{}\n", name);
            }
        }
        _ => panic!("Unknown mode"),
    }
    close(fd as usize);
}
