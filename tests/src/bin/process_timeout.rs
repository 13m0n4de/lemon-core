#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
    process::{exec, fork, get_time, waitpid, waitpid_nb, yield_},
    signal::{kill, SignalFlags},
};

#[no_mangle]
pub extern "Rust" fn main(argc: usize, argv: &[&str]) -> i32 {
    let timeout_ms = argv[1]
        .parse::<isize>()
        .expect("Error when parsing timeout!");

    let pid = fork();
    if pid == 0 {
        if exec(argv[2], &argv[2..argc]) == -1 {
            println!("Error when executing '{}'", argv[2]);
            return 1;
        }
    } else {
        let start_time = get_time();
        let mut exit_code: i32 = Default::default();
        while get_time() - start_time < timeout_ms {
            if waitpid_nb(pid as usize, &mut exit_code) == pid {
                println!(
                    "child exited in {}ms, exit_code = {}",
                    get_time() - start_time,
                    exit_code,
                );
                return 2;
            }
            yield_();
        }

        println!("child has run for {}ms, kill it!", timeout_ms);
        kill(pid as usize, SignalFlags::SIGINT.bits());
        assert_eq!(waitpid(pid as usize, &mut exit_code), pid);
        println!("exit code of the child is {}", exit_code);
    }

    0
}
