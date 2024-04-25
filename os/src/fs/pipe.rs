use alloc::sync::{Arc, Weak};

use super::File;
use crate::{mm::UserBuffer, sync::UPSafeCell, task::suspend_current_and_run_next};

/// Represents a unidirectional communication pipe with separate read and write ends.
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<UPSafeCell<PipeRingBuffer>>,
}

impl Pipe {
    /// Creates a read end of a pipe with a shared buffer.
    pub fn read_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
        }
    }

    /// Creates a write end of a pipe with a shared buffer.
    pub fn write_end_with_buffer(buffer: Arc<UPSafeCell<PipeRingBuffer>>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
        }
    }
}

impl File for Pipe {
    fn is_readable(&self) -> bool {
        self.readable
    }

    fn is_writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> usize {
        assert!(self.is_readable());
        let mut buf_iter = buf.iter_mut();
        let mut already_read = 0usize;

        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_to_read = ring_buffer.available_to_read();

            if available_to_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return already_read;
                }
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }

            for _ in 0..available_to_read {
                if let Some(byte_ref) = buf_iter.next() {
                    unsafe { *byte_ref = ring_buffer.read_byte() };
                    already_read += 1;
                } else {
                    return already_read;
                }
            }
        }
    }

    fn write(&self, mut buf: UserBuffer) -> usize {
        assert!(self.is_writable());
        let mut buf_iter = buf.iter_mut();
        let mut already_write = 0usize;

        loop {
            let mut ring_buffer = self.buffer.exclusive_access();
            let available_to_write = ring_buffer.available_to_write();

            if available_to_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }

            for _ in 0..available_to_write {
                if let Some(byte_ref) = buf_iter.next() {
                    ring_buffer.write_byte(unsafe { *byte_ref });
                    already_write += 1;
                } else {
                    return already_write;
                }
            }
        }
    }
}

const RING_BUFFER_SIZE: usize = 32;

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    Full,
    Empty,
    Normal,
}

/// Implements a ring buffer for data transfer between the read and write ends of a pipe.
#[allow(clippy::module_name_repetitions)]
pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::Empty,
            write_end: None,
        }
    }

    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.status = RingBufferStatus::Normal;
        self.arr[self.tail] = byte;
        self.tail = (self.tail + 1) % RING_BUFFER_SIZE;
        if self.tail == self.head {
            self.status = RingBufferStatus::Full;
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::Normal;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::Empty;
        }
        c
    }

    pub fn available_to_read(&self) -> usize {
        if self.status == RingBufferStatus::Empty {
            0
        } else if self.tail > self.head {
            self.tail - self.head
        } else {
            self.tail + RING_BUFFER_SIZE - self.head
        }
    }

    pub fn available_to_write(&self) -> usize {
        if self.status == RingBufferStatus::Full {
            0
        } else {
            RING_BUFFER_SIZE - self.available_to_read()
        }
    }

    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

/// Creates a pair of connected pipe ends, return (`read_end`, `write_end`).
pub fn make() -> (Arc<Pipe>, Arc<Pipe>) {
    let buffer = Arc::new(unsafe { UPSafeCell::new(PipeRingBuffer::new()) });
    let read_end = Arc::new(Pipe::read_end_with_buffer(buffer.clone()));
    let write_end = Arc::new(Pipe::write_end_with_buffer(buffer.clone()));
    buffer.exclusive_access().set_write_end(&write_end);
    (read_end, write_end)
}
