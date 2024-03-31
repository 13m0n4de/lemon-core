//! Process Management System Calls

use alloc::{sync::Arc, vec::Vec};
use log::trace;

use crate::{
    fs::{get_full_path, open_file, OpenFlags},
    mm::{translated_mut_ref, translated_ref, translated_str},
    task::{
        current_pcb, current_user_token, exit_current_and_run_next, pid2process,
        suspend_current_and_run_next, SignalFlags,
    },
    timer::get_time_ms,
};

/// Exits the current task and submits an exit code.
///
/// # Arguments
///
/// - `exit_code`: The exit code to be submitted on task exit.
///
/// # Panics
///
/// Panics if somehow reached, as it should exit the current task and not return.
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// Yields the current task's execution resources to other tasks.
///
/// # Returns
///
/// Always returns `0` to indicate successful yield.
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

/// Retrieves the current system time in milliseconds.
///
/// # Returns
///
/// The current system time, representing the time in milliseconds.
/// For more details, see [`get_time_ms`].
pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

/// Retrieves the Process ID (PID) of the current process.
///
/// # Returns
///
/// The PID of the current process.
pub fn sys_getpid() -> isize {
    current_pcb().pid() as isize
}

/// Creates a duplicate of the current process.
///
/// # Returns
///
/// Returns the Process ID (PID) of the newly created process to the parent process,
/// and `0` to the child process.
pub fn sys_fork() -> isize {
    let current_process = current_pcb();
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

/// Replaces the current process's image with a new process image.
///
/// This system call loads a new program into the current process's memory space
/// and starts its execution. The current process is completely replaced by the new program.
///
/// # Arguments
///
/// - `path`: A pointer to the null-terminated string representing the file path of the new program.
/// - `args`: A pointer to the array of arguments for the new program.
///
/// # Returns
///
/// - The number of arguments (`argc`) on success.
/// - `-1` if the file cannot be opened.
pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let process = current_pcb();
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

/// Waits for a child process to change state.
///
/// # Arguments
///
/// - `pid`: The PID of the child process. If `-1`, waits for any child process.
/// - `exit_code_ptr`: A pointer to where the exit code of the child process will be stored.
///
/// # Returns
///
/// - The PID of the child process if it has exited.
/// - `-1` if no matching child process exists.
/// - `-2` if the child process is still running.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let process = current_pcb();
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

/// Sends a signal to a process.
///
/// # Arguments
///
/// - `pid`: The PID of the process to signal.
/// - `signal`: The signal to send.
///
/// # Returns
///
/// - `0` on successfully sending the signal.
/// - `-1` if the specified process does not exist or the signal is invalid.
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
