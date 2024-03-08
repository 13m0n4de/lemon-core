//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};
use bitflags::*;

bitflags! {
    /// [`PageTableEntry`] flags
    #[derive(PartialEq)]
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

/// Page Table Entry
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PageTableEntry {
    bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self {
            bits: ppn.0 << 10 | flags.bits() as usize,
        }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }

    pub fn is_readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn is_writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn is_executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// Page Table
/// - `root_ppn`: The physical page number of the root of the page table
/// - `data_frames`: Physical frames for the data
/// - `metadata_frames`: Physical frames for the page table itself and its directory entries
pub struct PageTable {
    root_ppn: PhysPageNum,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    metadata_frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            data_frames: BTreeMap::new(),
            metadata_frames: vec![frame],
        }
    }
    /// Inserts a mapping for a [`VirtPageNum`] to a [`FrameTracker`], replacing any existing mapping, and returns the old frame if it existed.
    pub fn insert(&mut self, vpn: VirtPageNum, frame: FrameTracker) -> Option<FrameTracker> {
        self.data_frames.insert(vpn, frame)
    }

    /// Removes and returns the frame mapping for a [`VirtPageNum`] if it exists.
    pub fn remove(&mut self, vpn: &VirtPageNum) -> Option<FrameTracker> {
        self.data_frames.remove(vpn)
    }

    fn find_pte_then_alloc(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;

        for &idx in idxs[..2].iter() {
            let pte = ppn.as_mut_pte_array().get_mut(idx)?;
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.metadata_frames.push(frame);
            }
            ppn = pte.ppn();
        }
        ppn.as_mut_pte_array().get_mut(idxs[2])
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;

        for &idx in idxs[..2].iter() {
            let pte = ppn.as_mut_pte_array().get_mut(idx)?;
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }

        ppn.as_mut_pte_array().get_mut(idxs[2])
    }

    /// Insert a key-value pair into the multi-level page table
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_then_alloc(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// Remove a key-value pair from the multi-level page table
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            data_frames: BTreeMap::new(),
            metadata_frames: vec![],
        }
    }

    /// Translates a [`VirtPageNum`] to a [`PageTableEntry`] if it exists.
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    /// Translates a [`VirtAddr`] to a [`PhysAddr`]
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().to_vpn_by_floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }

    /// Generates a token representing the physical address of the page table
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();

    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.to_vpn_by_floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));

        if end_va.page_offset() == 0 {
            v.push(&mut ppn.as_mut_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.as_mut_bytes_array()[start_va.page_offset()..end_va.page_offset()])
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
