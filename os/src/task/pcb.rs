use core::cell::RefMut;

use crate::{mm::MemorySet, sync::UPSafeCell};

use super::id::{PidHandle, RecycleAllocator};

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
}

pub struct ProcessControlBlockInner {
    pub memory_set: MemorySet,
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
