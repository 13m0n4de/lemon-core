//! Process management syscalls

use alloc::{sync::Arc, vec::Vec};
use log::*;

use crate::{
    fs::{get_full_path, open_file, OpenFlags},
    mm::{translated_mut_ref, translated_ref, translated_str},
    task::{
        current_process, current_user_token, exit_current_and_run_next, pid2process,
        suspend_current_and_run_next, SignalFlags,
    },
    timer::get_time_ms,
};

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// get time in milliseconds
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_process().pid() as isize
}

pub fn sys_fork() -> isize {
    let current_process = current_process();
    let new_process = current_process.fork();
    let new_pid = new_process.pid();
    // modify trap context of new_task, because it returns immediately after switching
    let new_process_inner = new_process.inner_exclusive_access();
    let task = new_process_inner.tasks[0].as_ref().unwrap();
    let trap_cx = task.inner_exclusive_access().trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    new_pid as isize
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    let path = translated_str(token, path);
    let path = get_full_path(&process_inner.cwd, &path);
    drop(process_inner);

    let mut args_vec = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        let arg_str = translated_str(token, arg_str_ptr as *const u8);
        args_vec.push(arg_str);
        unsafe {
            args = args.add(1);
        }
    }

    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let data = app_inode.read_all();
        let argc = args_vec.len();
        process.exec(data.as_slice(), &args_vec);
        // return argc because cx.x[10] will be covered with it later
        argc as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let process = current_process();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = process.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.pid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB lock exclusively
        p.inner_exclusive_access().is_zombie && (pid == -1 || pid as usize == p.pid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.pid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_mut_ref(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}

pub fn sys_kill(pid: usize, signal: u32) -> isize {
    if let Some(process) = pid2process(pid) {
        if let Some(flag) = SignalFlags::from_bits(signal) {
            process.inner_exclusive_access().signals |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

// pub fn sys_sigaction(
//     signum: i32,
//     action: *const SignalAction,
//     old_action: *mut SignalAction,
// ) -> isize {
//     let token = current_user_token();
//     let task = current_task().unwrap();
//     let mut inner = task.inner_exclusive_access();

//     if signum as usize > MAX_SIG || action.is_null() || old_action.is_null() {
//         return -1;
//     }

//     if let Some(flag) = SignalFlags::from_bits(1 << signum) {
//         if flag == SignalFlags::SIGKILL || flag == SignalFlags::SIGSTOP {
//             return -1;
//         }
//         let prev_action = inner.signal_actions.table[signum as usize];
//         *translated_mut_ref(token, old_action) = prev_action;
//         inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
//         0
//     } else {
//         -1
//     }
// }

// pub fn sys_sigreturn() -> isize {
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         inner.handling_sig = None;
//         // restore the trap context
//         let trap_ctx = inner.trap_cx();
//         *trap_ctx = inner.trap_ctx_backup.unwrap();
//         // Here we return the value of a0 in the trap_ctx,
//         // otherwise it will be overwritten after we trap
//         // back to the original execution of the application.
//         trap_ctx.x[10] as isize
//     } else {
//         -1
//     }
// }

// pub fn sys_sigprocmask(mask: u32) -> isize {
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         let old_mask = inner.signal_mask;
//         if let Some(flag) = SignalFlags::from_bits(mask) {
//             inner.signal_mask = flag;
//             old_mask.bits() as isize
//         } else {
//             -1
//         }
//     } else {
//         -1
//     }
// }
