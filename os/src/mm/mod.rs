mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VPNRange, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use page_table::{PageTable, PageTableEntry};

/// Initiate heap allocator
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    frame_allocator::frame_allocator_test();
}
