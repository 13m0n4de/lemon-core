//! Implementation of [`MapArea`] and [`MemorySet`].

use super::{
    frame_allocator, PTEFlags, PageTable, PageTableEntry, PhysAddr, PhysPageNum, StepByOne,
    VPNRange, VirtAddr, VirtPageNum,
};
use crate::{
    config::MMIO,
    config::{MEMORY_END, PAGE_SIZE, TRAMPOLINE},
    sync::UPIntrFreeCell,
};
use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::arch::asm;
use lazy_static::lazy_static;
use log::{info, trace};
use riscv::register::satp;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    /// The memory set instance of kernel space
    pub static ref KERNEL_SPACE: Arc<UPIntrFreeCell<MemorySet>> =
        Arc::new(unsafe { UPIntrFreeCell::new(MemorySet::new_kernel()) });
}

///Get kernelspace root ppn
pub fn kernel_token() -> usize {
    KERNEL_SPACE.exclusive_access().token()
}

/// Map type for memory set: `identical` or `framed`
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
    /// offset of page num
    Linear(isize),
}

bitflags! {
    /// Map permission corresponding to that in pte: `R W X U`
    #[derive(Copy, Clone, PartialEq, Debug)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

/// Map area structure, controls a contiguous piece of virtual memory
pub struct MapArea {
    vpn_range: VPNRange,
    map_type: MapType,
    map_perm: MapPermission,
}

impl Clone for MapArea {
    fn clone(&self) -> Self {
        Self {
            vpn_range: VPNRange::new(self.vpn_range.start(), self.vpn_range.end()),
            map_type: self.map_type,
            map_perm: self.map_perm,
        }
    }
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn = start_va.as_vpn_by_floor();
        let end_vpn = end_va.as_vpn_by_ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            map_type,
            map_perm,
        }
    }

    /// Maps a single virtual page to a physical page based on the [`MapType`] and [`MapPermission`].
    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn = match self.map_type {
            MapType::Identical => PhysPageNum(vpn.0),
            MapType::Framed => {
                let frame = frame_allocator::alloc().unwrap();
                let frame_ppn = frame.ppn;
                page_table.insert(vpn, frame);
                frame_ppn
            }
            MapType::Linear(pn_offset) => {
                assert!(vpn.0 < (1usize << 27)); // check for sv39
                PhysPageNum((vpn.0 as isize + pn_offset) as usize)
            }
        };
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    // Unmaps a single virtual page from the page table, freeing associated resources if framed.
    #[allow(unused)]
    fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            page_table.remove(vpn);
        }
        page_table.unmap(vpn);
    }

    /// Maps all pages within the VPN range of this [`MapArea`] to physical pages.
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }

    /// Unmaps all pages within the VPN range of this [`MapArea`], potentially freeing resources.
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }

    /// Copies data into the virtual pages managed by this `MapArea`, assuming the area is framed.
    /// data: start-aligned but maybe with shorter length, assume that all frames were cleared before.
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);

        if data.is_empty() {
            return;
        }

        let chunk_size = PAGE_SIZE.min(data.len());
        let mut current_vpn = self.vpn_range.start();

        for src_chunk in data.chunks(chunk_size) {
            let ppn = page_table.translate(current_vpn).unwrap().ppn();
            let dst_bytes = ppn.as_mut_bytes_array();
            let copy_len = src_chunk.len().min(dst_bytes.len());
            dst_bytes[..copy_len].copy_from_slice(&src_chunk[..copy_len]);
            current_vpn.step();
        }
    }
}

/// Memory set structure, controls virtual-memory space
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl Clone for MemorySet {
    fn clone(&self) -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // copy data sections/trap_context/user_stack
        for area in &self.areas {
            let new_area = area.clone();
            memory_set.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = self.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .as_mut_bytes_array()
                    .copy_from_slice(src_ppn.as_mut_bytes_array());
            }
        }
        memory_set
    }
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    /// Activate SV39 paging mode
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }

    /// Translates a [`VirtPageNum`] to a [`PageTableEntry`] if it exists.
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    /// Generates a token representing the physical address of the page table
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// Add a new `MapArea` into this [`MemorySet`]
    /// Assuming that there are no conflicts in the virtual address space.
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }

    /// Assume that no conflicts
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    /// Remove [`MapArea`] that starts with `start_vpn`
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }

    /// Remove all [`MapArea`]
    pub fn recycle_data_pages(&mut self) {
        self.areas.clear();
    }

    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// Without kernel stacks.
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();

        memory_set.map_trampoline();

        info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        info!(
            ".bss [{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );

        trace!("mapping .text section");
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );

        trace!("mapping .rodata section");
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );

        trace!("mapping .data section");
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        trace!("mapping .bss section");
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        trace!("mapping physical memory");
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        trace!("mapping memory-mapped registers");
        for &pair in MMIO {
            memory_set.push(
                MapArea::new(
                    pair.0.into(),
                    (pair.0 + pair.1).into(),
                    MapType::Identical,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }

        memory_set
    }

    /// Include sections in elf and trampoline and `TrapContext` and user stack.
    /// Returns `user_sp` and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();

        memory_set.map_trampoline();

        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");

        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);

        // map program headers of elf, with U flag
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();

                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }

                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }

        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_base: usize = max_end_va.into();
        user_stack_base += PAGE_SIZE;

        (
            memory_set,
            user_stack_base,
            elf.header.pt2.entry_point() as usize,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{test, test_assert};

    test!(test_memory_set_kernel, {
        for vpn in VPNRange::new(
            VirtAddr::from(stext as usize).as_vpn_by_ceil(),
            VirtAddr::from(MEMORY_END).as_vpn_by_ceil(),
        ) {
            let pte_option = KERNEL_SPACE.exclusive_access().translate(vpn);
            test_assert!(matches!(pte_option, Some(pte) if pte.ppn().0 == vpn.0 ));
        }
        Ok("passed")
    });

    test!(test_memory_set_remap, {
        let kernel_space = KERNEL_SPACE.exclusive_access();
        let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
        let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
        let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

        test_assert!(!kernel_space
            .page_table
            .translate(mid_text.as_vpn_by_floor())
            .unwrap()
            .is_writable());

        test_assert!(!kernel_space
            .page_table
            .translate(mid_rodata.as_vpn_by_floor())
            .unwrap()
            .is_writable());

        test_assert!(!kernel_space
            .page_table
            .translate(mid_data.as_vpn_by_floor())
            .unwrap()
            .is_executable());

        Ok("passed")
    });

    test!(test_memory_set_clone, {
        let mut memory_set = MemorySet::new_bare();
        let data = [u8::MAX; PAGE_SIZE];
        memory_set.push(
            MapArea::new(
                VirtPageNum(0).into(),
                VirtPageNum(2).into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            Some(&data),
        );

        let new_memory_set = memory_set.clone();
        let pte_option = new_memory_set.translate(VirtPageNum(0));
        test_assert!(pte_option.is_some());
        for byte in pte_option.unwrap().ppn().as_mut_bytes_array() {
            test_assert!(*byte == u8::MAX);
        }

        let pte_option = new_memory_set.translate(VirtPageNum(1));
        test_assert!(pte_option.is_some());
        for byte in pte_option.unwrap().ppn().as_mut_bytes_array() {
            test_assert!(*byte == 0);
        }

        Ok("passed")
    });
}
