//! # Task Management

mod context;
mod control_block;
mod manager;
mod pid;
mod processor;
mod switch;

use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::loader::get_app_data_by_name;

use context::TaskContext;
use control_block::TaskControlBlock;
use processor::{schedule, take_current_task};

pub use manager::add_task;
pub use processor::{current_task, current_trap_cx, current_user_token};

#[derive(Copy, Clone, PartialEq)]
enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

lazy_static! {
    static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
}

/// Add init process to the mannger
pub fn add_initproc() {
    add_task(INITPROC.clone())
}

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

pub fn exit_current_and_run_next() {
    todo!()
}
