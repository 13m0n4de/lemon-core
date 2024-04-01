//! Implementation of [`Processor`]

use super::{context::Context, fetch_task, switch::__switch, tcb::TaskControlBlock, Status};
use crate::{sync::UPSafeCell, trap::Context as TrapContext};
use alloc::sync::Arc;
use lazy_static::lazy_static;

/// Processor management structure
pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_cx: Context,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: Context::zero_init(),
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
    fn idle_task_cx_ptr(&mut self) -> *mut Context {
        core::ptr::from_mut(&mut self.idle_task_cx)
    }
}

lazy_static! {
    static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

pub fn take_current_tcb() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

pub fn current_tcb() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

pub fn current_user_token() -> usize {
    let task = current_tcb().unwrap();
    let token = task.inner_exclusive_access().user_token();
    token
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    current_tcb().unwrap().inner_exclusive_access().trap_cx()
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
            let next_task_cx_ptr = &task_inner.task_cx as *const Context;
            task_inner.task_status = Status::Running;
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
pub fn schedule(switched_task_cx_ptr: *mut Context) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
