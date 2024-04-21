//! Implementation of [`FrameAllocator`]

use super::{PhysAddr, PhysPageNum};
use crate::{config::MEMORY_END, sync::UPSafeCell};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::lazy_static;

/// Manage a frame which has the same lifecycle as the tracker
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_array = ppn.as_mut_bytes_array();
        bytes_array.fill(0);
        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        dealloc(self.ppn);
    }
}

trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

/// An implementation for frame allocator
#[allow(clippy::module_name_repetitions)]
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        match self.recycled.pop() {
            Some(ppn) => Some(ppn.into()),
            None if self.current == self.end => None,
            None => {
                self.current += 1;
                Some((self.current - 1).into())
            }
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // validity check
        assert!(
            !(ppn >= self.current || self.recycled.iter().any(|&v| v == ppn)),
            "Frame ppn={ppn:#x} has not been allocated!"
        );
        // recycle
        self.recycled.push(ppn);
    }
}

type FrameAllocatorImpl = StackFrameAllocator;

lazy_static! {
    /// FrameAllocator global instance
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> =
        unsafe { UPSafeCell::new(FrameAllocatorImpl::new()) };
}

/// Initialize the frame allocator with `ekernel` and [`MEMORY_END`]
pub fn init() {
    extern "C" {
        fn ekernel();
    }

    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).as_ppn_by_ceil(),
        PhysAddr::from(MEMORY_END).as_ppn_by_floor(),
    );
}

/// Allocate a frame
pub fn alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

/// Deallocate a frame
pub fn dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}

/// A simple test for frame allocator
#[allow(unused)]
pub fn test() {
    let mut v: Vec<FrameTracker> = Vec::new();

    for i in 0..5 {
        let frame = alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    v.clear();

    for i in 0..5 {
        let frame = alloc().unwrap();
        println!("{:?}", frame);
        v.push(frame);
    }
    drop(v);

    println!("frame_allocator_test passed!");
}
