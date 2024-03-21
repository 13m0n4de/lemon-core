use crate::drivers::{
    chardev::CharDevice,
    plic::{IntrTargetPriority, Plic},
    BLOCK_DEVICE, UART,
};

pub const CLOCK_FREQ: usize = 10000000;
pub const MEMORY_END: usize = 0x8800_0000;

// https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c
pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (0x0C00_0000, 0x21_0000), // VIRT_PLIC in virt machine
    (0x1000_0000, 0x00_9000), // VIRT_UART0 with GPU  in virt machine
];

pub type BlockDeviceImpl = crate::drivers::block::VirtIOBlock;
pub type CharDeviceImpl = crate::drivers::chardev::NS16550a<VIRT_UART>;

pub const VIRT_PLIC: usize = 0x0C00_0000;
pub const VIRT_UART: usize = 0x1000_0000;

pub fn init() {
    use riscv::register::sie;
    let mut plic = unsafe { Plic::new(VIRT_PLIC) };
    let hart_id: usize = 0;
    plic.set_threshold(hart_id, IntrTargetPriority::Supervisor, 0);
    plic.set_threshold(hart_id, IntrTargetPriority::Machine, 1);
    // irq nums: 5 keyboard, 6 mouse, 8 block, 10 uart
    for intr_src_id in [8, 10] {
        plic.enable(hart_id, IntrTargetPriority::Supervisor, intr_src_id);
        plic.set_priority(intr_src_id, 1);
    }
    unsafe {
        sie::set_sext();
    }
}

pub fn irq_handler() {
    let mut plic = unsafe { Plic::new(VIRT_PLIC) };
    let intr_src_id = plic.claim(0, IntrTargetPriority::Supervisor);
    match intr_src_id {
        8 => BLOCK_DEVICE.handle_irq(),
        10 => UART.handle_irq(),
        _ => panic!("unsupported IRQ {}", intr_src_id),
    }
    plic.complete(0, IntrTargetPriority::Supervisor, intr_src_id);
}
