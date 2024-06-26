use crate::{
    drivers::{chardev::CharDevice, UART},
    mm::UserBuffer,
};

use super::File;

///Standard input
pub struct Stdin;

///Standard output
pub struct Stdout;

impl File for Stdin {
    fn is_readable(&self) -> bool {
        true
    }

    fn is_writable(&self) -> bool {
        false
    }

    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        let ch = UART.read();
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }

    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

impl File for Stdout {
    fn is_readable(&self) -> bool {
        false
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }

    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in &user_buf.buffers {
            print!("{}", core::str::from_utf8(buffer).unwrap());
        }
        user_buf.len()
    }
}
