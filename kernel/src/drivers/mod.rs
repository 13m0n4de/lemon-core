//! Drivers

pub mod block;
pub mod bus;
pub mod chardev;
pub mod gpu;
pub mod input;
pub mod plic;

pub use block::BLOCK_DEVICE;
pub use chardev::UART;
pub use gpu::GPU_DEVICE;
pub use input::{KEYBOARD_DEVICE, MOUSE_DEVICE};
use log::debug;

use self::chardev::CharDevice;

pub fn init() {
    debug!("[kernel] init uart");
    UART.init();
    debug!("[kernel] init gpu");
    let _gpu = GPU_DEVICE.clone();
    debug!("[kernel] init keyboard");
    let _keyboard = KEYBOARD_DEVICE.clone();
    debug!("[kernel] init mouse");
    let _mouse = MOUSE_DEVICE.clone();
}
