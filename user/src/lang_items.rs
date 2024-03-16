use core::panic::PanicInfo;

use crate::{getpid, kill, signal::SignalFlags};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}, {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println!("Panicked: {}", info.message().unwrap());
    }
    kill(getpid() as usize, SignalFlags::SIGABRT.bits());
    unreachable!()
}
