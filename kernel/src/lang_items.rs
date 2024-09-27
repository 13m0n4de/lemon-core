//! # The Panic Handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;
use log::error;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message(),
        );
    } else {
        error!("[kernel] Panicked: {}", info.message());
    }
    shutdown(true)
}
