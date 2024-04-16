pub const CLOCK_FREQ: usize = 10_000_000;
pub const MEMORY_END: usize = 0x8800_0000;

// https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c
pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (0x1000_1000, 0x00_1000), // Virtio Block in virt machine
];

pub type BlockDeviceImpl = crate::drivers::block::virtio_blk::VirtIOBlock;
