use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{
    config::{kernel_stack_position, PAGE_SIZE, TRAP_CONTEXT_BASE, USER_STACK_SIZE},
    mm::{MapPermission, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
};

/// Bind pid lifetime to [`PidHandle`]
pub struct PidHandle(pub usize);

pub struct RecycleAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

impl RecycleAllocator {
    pub fn new() -> Self {
        RecycleAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }

    /// Allocate a id
    pub fn alloc(&mut self) -> usize {
        if let Some(id) = self.recycled.pop() {
            id
        } else {
            self.current += 1;
            self.current - 1
        }
    }

    /// Recycle a id
    pub fn dealloc(&mut self, id: usize) {
        assert!(id < self.current);
        assert!(
            !self.recycled.iter().any(|recycled_id| *recycled_id == id),
            "id {} has been deallocated!",
            id
        );
        self.recycled.push(id);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<RecycleAllocator> =
        unsafe { UPSafeCell::new(RecycleAllocator::new()) };
}

pub fn pid_alloc() -> PidHandle {
    PidHandle(PID_ALLOCATOR.exclusive_access().alloc())
}

/// Kernel stack for app
pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// Create a kernel stack from pid
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid: pid_handle.0 }
    }

    // Get the value on the top of kernel stack
    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }

    #[allow(unused)]
    /// Push a value on top of kernel stack
    pub fn push_on_top<T>(&self, value: T) -> *mut T
    where
        T: Sized,
    {
        let kernel_stack_top = self.top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe {
            *ptr_mut = value;
        }
        ptr_mut
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .exclusive_access()
            .remove_area_with_start_vpn(kernel_stack_bottom_va.into());
    }
}

fn trap_cx_bottom_from_tid(tid: usize) -> usize {
    TRAP_CONTEXT_BASE - tid * PAGE_SIZE
}

fn ustack_bottom_from_tid(ustack_base: usize, tid: usize) -> usize {
    ustack_base + tid * (PAGE_SIZE + USER_STACK_SIZE)
}
