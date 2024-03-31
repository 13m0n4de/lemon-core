use crate::drivers::{KEYBOARD_DEVICE, MOUSE_DEVICE, UART};

pub fn sys_event_get() -> isize {
    let keyboard = KEYBOARD_DEVICE.clone();
    let mouse = MOUSE_DEVICE.clone();

    if !keyboard.is_empty() {
        keyboard.read_event() as isize
    } else if !mouse.is_empty() {
        mouse.read_event() as isize
    } else {
        0
    }
}

pub fn sys_key_pressed() -> isize {
    isize::from(!UART.is_read_buffer_empty())
}
