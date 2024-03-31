use crate::process::yield_;
use crate::syscall::{sys_gettid, sys_thread_create, sys_waittid};

#[allow(clippy::module_name_repetitions)]
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
                let _ = yield_();
            }
            exit_code => return exit_code,
        }
    }
}
