//! App management syscalls
use log::*;

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    panic!("Unreachable in sys_exit!");
}
