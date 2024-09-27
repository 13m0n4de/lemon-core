use core::panic::PanicInfo;

use crate::println;
use crate::process::getpid;
use crate::signal::{kill, SignalFlags};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message());
    }
    kill(getpid() as usize, SignalFlags::SIGABRT.bits());
    unreachable!()
}
