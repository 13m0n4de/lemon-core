//! Implementation of syscalls

const SYSCALL_GETCWD: usize = 17;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP2: usize = 24;
const SYSCALL_MKDIR: usize = 34;
const SYSCALL_UNLINK: usize = 35;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_SLEEP: usize = 101;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_KILL: usize = 129;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_THREAD_CREATE: usize = 1000;
const SYSCALL_GETTID: usize = 1001;
const SYSCALL_WAITTID: usize = 1002;
const SYSCALL_MUTEX_CREATE: usize = 1010;
const SYSCALL_MUTEX_LOCK: usize = 1011;
const SYSCALL_MUTEX_UNLOCK: usize = 1012;
const SYSCALL_SEMAPHORE_CREATE: usize = 1020;
const SYSCALL_SEMAPHORE_UP: usize = 1021;
const SYSCALL_SEMAPHORE_DOWN: usize = 1022;
const SYSCALL_CONDVAR_CREATE: usize = 1030;
const SYSCALL_CONDVAR_SIGNAL: usize = 1031;
const SYSCALL_CONDVAR_WAIT: usize = 1032;
const SYSCALL_FRAMEBUFFER: usize = 2000;
const SYSCALL_FRAMEBUFFER_FLUSH: usize = 2001;
const SYSCALL_EVENT_GET: usize = 3000;
const SYSCALL_KEY_PRESSED: usize = 3001;

mod fs;
mod gui;
mod input;
mod process;
mod sync;
mod thread;

use fs::{
    sys_chdir, sys_close, sys_dup, sys_dup2, sys_fstat, sys_getcwd, sys_mkdir, sys_open, sys_pipe,
    sys_read, sys_unlink, sys_write,
};
use gui::{sys_framebuffer, sys_framebuffer_flush};
use input::{sys_event_get, sys_key_pressed};
use process::{
    sys_exec, sys_exit, sys_fork, sys_get_time, sys_getpid, sys_kill, sys_waitpid, sys_yield,
};
use sync::{
    sys_condvar_create, sys_condvar_signal, sys_condvar_wait, sys_mutex_create, sys_mutex_lock,
    sys_mutex_unlock, sys_semaphore_create, sys_semaphore_down, sys_semaphore_up, sys_sleep,
};
use thread::{sys_gettid, sys_thread_create, sys_waittid};

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_GETCWD => sys_getcwd(args[0] as *const u8, args[1]),
        SYSCALL_DUP => sys_dup(args[0]),
        SYSCALL_DUP2 => sys_dup2(args[0], args[1]),
        SYSCALL_MKDIR => sys_mkdir(args[0] as *const u8),
        SYSCALL_UNLINK => sys_unlink(args[0] as *const u8, args[1] as u32),
        SYSCALL_CHDIR => sys_chdir(args[0] as *const u8),
        SYSCALL_OPEN => sys_open(args[0] as *const u8, args[1] as u32),
        SYSCALL_CLOSE => sys_close(args[0]),
        SYSCALL_PIPE => sys_pipe(args[0] as *mut usize),
        SYSCALL_READ => sys_read(args[0], args[1] as *const u8, args[2]),
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_FSTAT => sys_fstat(args[0], args[1] as *mut u8),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_SLEEP => sys_sleep(args[0]),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_KILL => sys_kill(args[0], args[1] as u32),
        SYSCALL_GET_TIME => sys_get_time(),
        SYSCALL_GETPID => sys_getpid(),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8, args[1] as *const usize),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1] as *mut i32),
        SYSCALL_THREAD_CREATE => sys_thread_create(args[0], args[1]),
        SYSCALL_GETTID => sys_gettid(),
        SYSCALL_WAITTID => sys_waittid(args[0]) as isize,
        SYSCALL_MUTEX_CREATE => sys_mutex_create(args[0] == 1),
        SYSCALL_MUTEX_LOCK => sys_mutex_lock(args[0]),
        SYSCALL_MUTEX_UNLOCK => sys_mutex_unlock(args[0]),
        SYSCALL_SEMAPHORE_CREATE => sys_semaphore_create(args[0]),
        SYSCALL_SEMAPHORE_UP => sys_semaphore_up(args[0]),
        SYSCALL_SEMAPHORE_DOWN => sys_semaphore_down(args[0]),
        SYSCALL_CONDVAR_CREATE => sys_condvar_create(),
        SYSCALL_CONDVAR_SIGNAL => sys_condvar_signal(args[0]),
        SYSCALL_CONDVAR_WAIT => sys_condvar_wait(args[0], args[1]),
        SYSCALL_FRAMEBUFFER => sys_framebuffer(),
        SYSCALL_FRAMEBUFFER_FLUSH => sys_framebuffer_flush(),
        SYSCALL_EVENT_GET => sys_event_get(),
        SYSCALL_KEY_PRESSED => sys_key_pressed(),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
