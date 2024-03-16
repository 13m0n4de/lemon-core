use crate::sync::UPSafeCell;

use super::id::{PidHandle, RecycleAllocator};

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    pub task_res_allocator: RecycleAllocator,
}

impl ProcessControlBlockInner {
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }
}
