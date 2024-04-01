//! # The Panic Handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;
use log::error;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let message = info.message().unwrap();
    if let Some(location) = info.location() {
        error!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            message
        );
    } else {
        error!("Panicked: {}", message);
    }
    shutdown(true)
}
