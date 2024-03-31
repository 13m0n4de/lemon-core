//! Implementation of [`Processor`]

use super::{
    context::Context, manager::fetch, pcb::ProcessControlBlock, switch::__switch, tcb::Status,
    ControlBlock,
};
use crate::{sync::UPIntrFreeCell, trap::Context as TrapContext};
use alloc::sync::Arc;
use lazy_static::lazy_static;

/// Processor management structure
pub struct Processor {
    current: Option<Arc<ControlBlock>>,
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
    pub fn take_current(&mut self) -> Option<Arc<ControlBlock>> {
        self.current.take()
    }

    /// Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<ControlBlock>> {
        self.current.clone()
    }

    /// Get mutable reference to `idle_task_cx`
    fn idle_task_cx_ptr(&mut self) -> *mut Context {
        core::ptr::from_mut(&mut self.idle_task_cx)
    }
}

lazy_static! {
    static ref PROCESSOR: UPIntrFreeCell<Processor> =
        unsafe { UPIntrFreeCell::new(Processor::new()) };
}

/// take the thread that the current processor is executing
pub fn take_current_tcb() -> Option<Arc<ControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Current TCB
pub fn current_tcb() -> Option<Arc<ControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Current PCB
pub fn current_pcb() -> Arc<ProcessControlBlock> {
    current_tcb().unwrap().process.upgrade().unwrap()
}

/// Current satp
pub fn current_user_token() -> usize {
    let task = current_tcb().unwrap();
    task.user_token()
}

/// Current trap context
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_tcb().unwrap().inner_exclusive_access().trap_cx()
}

/// Virtual address of current trap context
pub fn current_trap_cx_user_va() -> usize {
    current_tcb()
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
        if let Some(task) = fetch() {
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
