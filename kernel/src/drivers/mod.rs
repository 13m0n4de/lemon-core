//! Drivers

pub mod block;
pub mod bus;
pub mod chardev;
pub mod gpu;
pub mod plic;

pub use block::BLOCK_DEVICE;
pub use chardev::UART;
pub use gpu::GPU_DEVICE;

use self::chardev::CharDevice;

pub fn init() {
    UART.init();
    let _gpu = GPU_DEVICE.clone();
}
