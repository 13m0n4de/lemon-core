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

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use memory_set::{kernel_token, MapArea, MapPermission, MapType, MemorySet, KERNEL_SPACE};
pub use page_table::{PTEFlags, PageTable, PageTableEntry};

use address::VPNRange;
use alloc::{string::String, vec::Vec};

/// Initiate heap allocator, frame allocator, kernel space.
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();

    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.as_vpn_by_floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));

        if end_va.page_offset() == 0 {
            v.push(&mut ppn.as_mut_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.as_mut_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }

        start = end_va.into();
    }

    v
}

/// Load a string from other address spaces into kernel space without an end `\0`.
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .as_mut_ref());
        if ch == 0 {
            break;
        }
        string.push(ch as char);
        va += 1;
    }
    string
}

pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .as_ref()
}

///translate a generic through page table and return a mutable reference
pub fn translated_mut_ref<T>(token: usize, ptr: *mut T) -> &'static mut T {
    //println!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    //println!("translated_refmut: before translate_va");
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .as_mut_ref()
}

/// Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    /// Create a [`UserBuffer`] by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }

    /// Length of [`UserBuffer`]
    pub fn len(&self) -> usize {
        self.buffers.iter().map(|b| b.len()).sum()
    }

    #[allow(unused)]
    pub fn iter(&self) -> impl Iterator<Item = *const u8> + '_ {
        self.buffers
            .iter()
            .flat_map(|buffer| buffer.iter().map(core::ptr::from_ref))
    }

    #[allow(unused)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = *mut u8> + '_ {
        self.buffers
            .iter_mut()
            .flat_map(|buffer| buffer.iter_mut().map(core::ptr::from_mut))
    }
}
