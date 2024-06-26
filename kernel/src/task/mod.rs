//! # Task Management

mod context;
mod id;
pub mod manager;
pub mod pcb;
mod processor;
mod signal;
mod switch;
pub mod tcb;

use crate::{
    fs::{open_file, OpenFlags},
    sbi::shutdown,
};
use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use log::info;

pub use context::Context;
pub use manager::{pid2process, remove_from_pid2process};
pub use processor::{
    current_pcb, current_tcb, current_trap_cx, current_trap_cx_user_va, current_user_token,
    run_tasks, schedule, take_current_tcb,
};
pub use signal::{add_signal_to_current, check_signals_error_of_current, SignalFlags};

use id::TaskUserRes;
use pcb::ProcessControlBlock;
use tcb::Status;

lazy_static! {
    /// Global process that init user shell
    pub static ref DAEMON: Arc<ProcessControlBlock> = {
        let inode = open_file("/bin/daemon", OpenFlags::RDONLY).expect("Failed to open '/bin/daemon'.");
        let v = inode.read_all();
        ProcessControlBlock::new(v.as_slice())
    };
}

/// Add init process to the manager
pub fn init() {
    let _daemon = DAEMON.clone();
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Suspend the current 'Running' task and run the next task in task list
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_tcb().unwrap();

    // --- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut Context;
    // change status to Ready
    task_inner.task_status = Status::Ready;
    drop(task_inner);
    // --- release current TCB

    // push back to ready queue.
    manager::add(task);
    // jump to scheduling cycle.
    schedule(task_cx_ptr);
}

pub fn block_current() -> *mut Context {
    let task = take_current_tcb().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = Status::Blocked;
    core::ptr::from_mut::<Context>(&mut task_inner.task_cx)
}

pub fn block_current_and_run_next() {
    let task_cx_ptr = block_current();
    schedule(task_cx_ptr);
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_tcb().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let process = task.process.upgrade().unwrap();
    let tid = task_inner.res.as_ref().unwrap().tid;

    // record exit code and recycle task user res
    task_inner.exit_code = Some(exit_code);
    task_inner.res = None;
    // here we do not remove the thread since we are still using the kstack
    // it will be deallocated when sys_waittid is called
    drop(task_inner);
    drop(task);

    // however, if this is the main thread of current process
    // the process should terminate at once

    if tid == 0 {
        let pid = process.pid();
        if pid == IDLE_PID {
            info!(
                "[kernel] Idle process exit with exit_code {} ...",
                exit_code
            );
            if exit_code != 0 {
                shutdown(true)
            } else {
                shutdown(false)
            }
        }

        // remove from pid2process
        remove_from_pid2process(pid);

        let mut process_inner = process.inner_exclusive_access();
        // mark this process as a zombie process
        process_inner.is_zombie = true;
        // record exit code of main process
        process_inner.exit_code = exit_code;

        {
            // move all child processes under daemon process
            let mut daemon_inner = DAEMON.inner_exclusive_access();
            for child in &process_inner.children {
                child.inner_exclusive_access().parent = Some(Arc::downgrade(&DAEMON));
                daemon_inner.children.push(child.clone());
            }
        }

        // deallocate user res (including tid/trap_cx/ustack) of all threads
        // it has to be done before we dealloc the whole memory_set
        // otherwise they will be deallocated twice
        let mut recycle_res = Vec::<TaskUserRes>::new();
        for task in process_inner.tasks.iter().filter(|t| t.is_some()) {
            let task = task.as_ref().unwrap();
            // if other tasks are Ready in TaskManager or waiting for a timer to be
            // expired, we should remove them.
            //
            // Mention that we do not need to consider Mutex/Semaphore since they
            // are limited in a single process. Therefore, the blocked tasks are
            // removed when the PCB is deallocated.
            let mut task_inner = task.inner_exclusive_access();
            if let Some(res) = task_inner.res.take() {
                recycle_res.push(res);
            }
        }
        // dealloc_tid and dealloc_user_res require access to PCB inner, so we
        // need to collect those user res first, then release process_inner
        // for now to avoid deadlock/double borrow problem.
        drop(process_inner);
        recycle_res.clear();

        let mut process_inner = process.inner_exclusive_access();
        process_inner.children.clear();
        // deallocate other data in user space i.e. program code/data section
        process_inner.memory_set.recycle_data_pages();
        // drop file descriptors
        process_inner.fd_table.clear();
        // Remove all tasks except for the main thread itself.
        // This is because we are still using the kstack under the TCB
        // of the main thread. This TCB, including its kstack, will be
        // deallocated when the process is reaped via waitpid.
        process_inner.tasks.truncate(1);
    }

    drop(process);
    // we do not have to save task context
    schedule(core::ptr::from_mut(&mut Context::zero_init()));
}
