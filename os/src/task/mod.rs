//! # Task Management

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
mod tcb;

use crate::{
    fs::{open_file, OpenFlags},
    sbi::shutdown,
};
use alloc::sync::Arc;
use context::Context;
use lazy_static::lazy_static;
use log::info;
use processor::{schedule, take_current_tcb};
use tcb::TaskControlBlock;

#[allow(clippy::module_name_repetitions)]
pub use manager::{add as add_task, fetch as fetch_task};
pub use processor::{current_tcb, current_trap_cx, current_user_token, run_tasks};

#[derive(Copy, Clone, PartialEq)]
pub enum Status {
    Ready,
    Running,
    Zombie,
}

lazy_static! {
    /// Global process that init user shell
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

/// Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
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
    add_task(task);
    // jump to scheduling cycle.
    schedule(task_cx_ptr);
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_tcb().unwrap();

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

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = Status::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in &inner.children {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
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
    schedule(core::ptr::from_mut(&mut Context::zero_init()));
}
