use alloc::vec::Vec;
use lazy_static::lazy_static;
use virtio_drivers::Hal;

use crate::{
    mm::{
        frame_alloc, frame_dealloc, kernel_token, FrameTracker, PageTable, PhysAddr, PhysPageNum,
        StepByOne, VirtAddr,
    },
    sync::UPIntrFreeCell,
};

lazy_static! {
    static ref QUEUE_FRAMES: UPIntrFreeCell<Vec<FrameTracker>> =
        unsafe { UPIntrFreeCell::new(Vec::new()) };
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let frame = frame_alloc().unwrap();
        let ppn_base = frame.ppn;
        QUEUE_FRAMES.exclusive_access().push(frame);

        for i in 1..pages {
            let frame = frame_alloc().unwrap();
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
            QUEUE_FRAMES.exclusive_access().push(frame);
        }

        let pa: PhysAddr = ppn_base.into();
        pa.0
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let pa = PhysAddr::from(pa);
        let mut ppn_base: PhysPageNum = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.step();
        }
        0
    }

    fn phys_to_virt(addr: usize) -> usize {
        addr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        PageTable::from_token(kernel_token())
            .translate_va(VirtAddr::from(vaddr))
            .unwrap()
            .0
    }
}