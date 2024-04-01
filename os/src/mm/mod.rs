//! # Memory management
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like:
//! - [`frame_allocator`]
//! - [`page_table`],
//! - [`memory_set::MapArea`]
//! - [`memory_set::MemorySet`]
//!
//! Every task or process has a [`memory_set::MemorySet`] to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
use page_table::{PTEFlags, PageTable};

/// Initiate heap allocator, fream allocator, kernel space.
pub fn init() {
    heap_allocator::init();
    frame_allocator::init();
    KERNEL_SPACE.exclusive_access().activate();
}
