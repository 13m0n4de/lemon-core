use super::{
    context::Context,
    id::{kstack_alloc, KernelStack, TaskUserRes},
    pcb::ProcessControlBlock,
};
use crate::{
    mm::PhysPageNum,
    sync::{UPIntrFreeCell, UPIntrRefMut},
    trap,
};
use alloc::sync::{Arc, Weak};

#[allow(clippy::module_name_repetitions)]
pub struct TaskControlBlock {
    pub process: Weak<ProcessControlBlock>,
    pub kstack: KernelStack,
    inner: UPIntrFreeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn new(
        process: &Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(process, ustack_base, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kstack = kstack_alloc();
        let kstack_top = kstack.top();
        Self {
            process: Arc::downgrade(process),
            kstack,
            inner: unsafe {
                UPIntrFreeCell::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: Context::leave_trap(kstack_top),
                    task_status: Status::Ready,
                    exit_code: None,
                })
            },
        }
    }

    pub fn inner_exclusive_access(&self) -> UPIntrRefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.inner_exclusive_access();
        inner.memory_set.token()
    }
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: Context,
    pub task_status: Status,
    pub exit_code: Option<i32>,
}

impl TaskControlBlockInner {
    pub fn trap_cx(&self) -> &'static mut trap::Context {
        self.trap_cx_ppn.as_mut_ref()
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Status {
    Ready,
    Running,
    Blocked,
}
