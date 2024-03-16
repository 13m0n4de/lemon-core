//! # Task Management

mod context;
mod id;
mod manager;
mod pcb;
mod processor;
mod signal;
mod switch;
mod tcb;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use log::*;

use crate::{
    fs::{find_inode, open_file, OpenFlags},
    sbi::shutdown,
};
use context::TaskContext;
use processor::{schedule, take_current_task};

pub use manager::{add_task, pid2task};
pub use processor::{current_task, current_trap_cx, current_user_token, run_tasks};
pub use signal::{
    add_signal_to_current, check_signals_error_of_current, handle_signals, SignalAction,
    SignalActions, SignalFlags, MAX_SIG,
};

use self::{manager::remove_from_pid2task, tcb::TaskControlBlock};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

lazy_static! {
    /// Global process that init user shell
    pub static ref DAEMON: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("/bin/daemon", OpenFlags::RDONLY).expect("Failed to open '/bin/daemon'.");
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

/// Add init process to the manager
pub fn init() {
    let root_inode = find_inode("/").expect("Failed to find inode for '/'.");
    let procs_inode = root_inode
        .create_dir("proc")
        .expect("Failed to create inode for '/proc/'.");
    procs_inode.set_default_dirent(root_inode.inode_id());
    add_task(DAEMON.clone());
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Suspend the current 'Running' task and run the next task in task list
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // --- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // --- release current TCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle.
    schedule(task_cx_ptr);
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
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

    // remove from pid2task
    remove_from_pid2task(task.getpid());
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under daemon proc

    // ++++++ access daemon TCB exclusively
    {
        let mut daemon_inner = DAEMON.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&DAEMON));
            daemon_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}
