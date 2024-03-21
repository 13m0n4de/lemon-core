//! Drivers

pub mod block;
pub mod chardev;
pub mod plic;

pub use block::BLOCK_DEVICE;
pub use chardev::UART;

use lazy_static::lazy_static;

use crate::sync::UPIntrFreeCell;

lazy_static! {
    pub static ref DEV_NON_BLOCKING_ACCESS: UPIntrFreeCell<bool> =
        unsafe { UPIntrFreeCell::new(false) };
}

pub fn init() {
    *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;
}
