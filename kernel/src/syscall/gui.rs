use crate::drivers::GPU_DEVICE;
use crate::mm::{MapArea, MapPermission, MapType, PhysAddr, VirtAddr};
use crate::task::current_pcb;

const FB_VADDR: usize = 0x1000_0000;

#[allow(clippy::similar_names)]
pub fn sys_framebuffer() -> isize {
    let fb = GPU_DEVICE.framebuffer();

    let fb_start_pa = PhysAddr::from(fb.as_ptr() as usize);
    assert!(fb_start_pa.is_aligned());

    let fb_start_ppn = fb_start_pa.as_ppn_by_floor();
    let fb_start_vpn = VirtAddr::from(FB_VADDR).as_vpn_by_floor();
    let pn_offset = fb_start_ppn.0 as isize - fb_start_vpn.0 as isize;

    let process = current_pcb();
    let mut process_inner = process.inner_exclusive_access();

    process_inner.memory_set.push(
        MapArea::new(
            (FB_VADDR).into(),
            (FB_VADDR + fb.len()).into(),
            MapType::Linear(pn_offset),
            MapPermission::R | MapPermission::W | MapPermission::U,
        ),
        None,
    );

    FB_VADDR as isize
}

pub fn sys_framebuffer_flush() -> isize {
    GPU_DEVICE.flush();
    0
}
