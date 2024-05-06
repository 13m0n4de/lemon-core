//! Thread Management System Calls

use alloc::sync::Arc;

use crate::{
    mm::kernel_token,
    task::{current_tcb, manager, tcb::TaskControlBlock},
    trap::{user_handler, Context},
};

/// Creates a new thread within the current process.
///
/// This function creates a new thread with a specific entry point and an argument. The new thread shares
/// the same address space as the calling thread but has its own stack and execution context. It is added
/// to the scheduler for execution.
///
/// # Arguments
///
/// - `entry`: The entry point address where the new thread starts execution.
/// - `arg`: An argument passed to the thread's entry point function.
///
/// # Returns
///
/// - The Thread ID (TID) of the newly created thread on success.
pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let task = current_tcb().unwrap();
    let process = task.process.upgrade().unwrap();

    // create a new thread
    let new_task = Arc::new(TaskControlBlock::new(
        &process,
        task.inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .ustack_base,
        true,
    ));

    // add new task to scheduler
    manager::add(new_task.clone());
    let new_task_inner = new_task.inner_exclusive_access();
    let new_task_res = new_task_inner.res.as_ref().unwrap();
    let new_task_tid = new_task_res.tid;
    let mut process_inner = process.inner_exclusive_access();

    // add new thread to current process
    let tasks = &mut process_inner.tasks;
    tasks.resize_with(new_task_tid + 1, || None);
    tasks[new_task_tid] = Some(new_task.clone());

    let new_task_trap_cx = new_task_inner.trap_cx();
    *new_task_trap_cx = Context::app_init_context(
        entry,
        new_task_res.ustack_top(),
        kernel_token(),
        new_task.kstack.top(),
        user_handler as usize,
    );
    new_task_trap_cx.x[10] = arg;
    new_task_tid as isize
}

/// Retrieves the Thread ID (TID) of the current thread.
///
/// # Returns
///
/// The TID of the current thread.
pub fn sys_gettid() -> isize {
    current_tcb()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid as isize
}

/// Waits for a thread within the same process to exit and retrieves its exit code.
///
/// This function blocks the calling thread until the specified thread exits. It is not possible
/// for a thread to wait on itself. If the specified thread has already exited, its exit code
/// is immediately returned and its resources are deallocated.
///
/// # Arguments
///
/// - `tid`: The TID of the thread to wait for.
///
/// # Returns
///
/// - The exit code of the waited thread on success.
/// - `-1` if the thread attempts to wait on itself or if the specified thread does not exist.
/// - `-2` if the specified thread has not yet exited.
pub fn sys_waittid(tid: usize) -> i32 {
    let task = current_tcb().unwrap();
    let process = task.process.upgrade().unwrap();
    let task_inner = task.inner_exclusive_access();
    let mut process_inner = process.inner_exclusive_access();

    // a thread cannot wait for itself
    if task_inner.res.as_ref().unwrap().tid == tid {
        return -1;
    }

    let mut exit_code: Option<i32> = None;
    let waited_task = process_inner.tasks[tid].as_ref();
    if let Some(waited_task) = waited_task {
        if let Some(waited_exit_code) = waited_task.inner_exclusive_access().exit_code {
            exit_code = Some(waited_exit_code);
        }
    } else {
        // waited thread does not exist
        return -1;
    }

    if let Some(exit_code) = exit_code {
        // dealloc the exited thread
        process_inner.tasks[tid] = None;
        exit_code
    } else {
        // waited thread has not exited
        -2
    }
}
