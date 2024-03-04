mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, StepByOne, VPNRange, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{PTEFlags, PageTable, PageTableEntry};

/// Initiate heap allocator
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
    memory_set::remap_test();
}
