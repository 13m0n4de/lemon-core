use crate::syscall::*;

pub fn sleep(sleep_ms: usize) {
    sys_sleep(sleep_ms);
}
