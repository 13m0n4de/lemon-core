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
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry};

use address::{StepByOne, VPNRange};
use page_table::{PTEFlags, PageTable};

/// Initiate heap allocator, fream allocator, kernel space.
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
