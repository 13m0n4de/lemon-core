use alloc::{collections::BTreeMap, vec, vec::Vec};
use bitflags::*;

use super::{frame_alloc, FrameTracker, PhysPageNum, VirtPageNum};

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
    pub bits: usize,
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

    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }

    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }

    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

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

    pub fn insert(&mut self, vpn: VirtPageNum, frame: FrameTracker) -> Option<FrameTracker> {
        self.data_frames.insert(vpn, frame)
    }

    pub fn remove(&mut self, vpn: &VirtPageNum) -> Option<FrameTracker> {
        self.data_frames.remove(vpn)
    }

    fn find_pte_then_alloc(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;

        for &idx in idxs[..2].iter() {
            let pte = ppn.get_pte_array().get_mut(idx)?;
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.metadata_frames.push(frame);
            }
            ppn = pte.ppn();
        }
        ppn.get_pte_array().get_mut(idxs[2])
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;

        for &idx in idxs[..2].iter() {
            let pte = ppn.get_pte_array().get_mut(idx)?;
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }

        ppn.get_pte_array().get_mut(idxs[2])
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_then_alloc(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & (1usize << 44) - 1),
            data_frames: BTreeMap::new(),
            metadata_frames: vec![],
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}
