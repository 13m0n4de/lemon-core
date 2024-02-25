//! # The Panic Handler

use crate::sbi::shutdown;
use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    let message = info.message().unwrap();
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            message
        );
    } else {
        println!("Panicked: {}", message);
    }
    shutdown(true)
}
