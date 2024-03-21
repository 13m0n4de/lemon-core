//! Implementation of [`Processor`]

use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{sync::UPIntrFreeCell, trap::TrapContext};

use super::{
    context::TaskContext, manager::fetch_task, pcb::ProcessControlBlock, switch::__switch,
    tcb::TaskStatus, TaskControlBlock,
};

/// Processor management structure
pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    /// Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    /// Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.clone()
    }

    /// Get mutable reference to `idle_task_cx`
    fn idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
}

lazy_static! {
    static ref PROCESSOR: UPIntrFreeCell<Processor> =
        unsafe { UPIntrFreeCell::new(Processor::new()) };
}

/// take the thread that the current processor is executing
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Current TCB
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Current PCB
pub fn current_process() -> Arc<ProcessControlBlock> {
    current_task().unwrap().process.upgrade().unwrap()
}

/// Current satp
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.user_token()
}

/// Current trap context
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().inner_exclusive_access().trap_cx()
}

/// Virtual address of current trap context
pub fn current_trap_cx_user_va() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .trap_cx_user_va()
}

/// The main part of process execution and scheduling.
/// Loop [`fetch_task`] to get the process that needs to run, and switch the process through
/// `__switch`
pub fn run_tasks() {
    loop {
        if let Some(task) = fetch_task() {
            let mut processor = PROCESSOR.exclusive_access();
            let idle_task_cx_ptr = processor.idle_task_cx_ptr();

            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);

            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);

            unsafe { __switch(idle_task_cx_ptr, next_task_cx_ptr) }
        }
    }
}

/// Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
