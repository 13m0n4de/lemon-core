//! # The Panic Handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;
use log::*;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let message = info.message().unwrap();
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            message
        );
    } else {
        error!("[kernel] Panicked: {}", message);
    }
    shutdown(true)
}
