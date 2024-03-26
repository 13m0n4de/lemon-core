//! ns16550a
//!
//! - Ref: <https://www.lammertbies.nl/comm/info/serial-uart>
//! - Ref: ns16550a datasheet: <https://datasheetspdf.com/pdf-file/605590/NationalSemiconductor/NS16550A/1>
//! - Ref: ns16450 datasheet: <https://datasheetspdf.com/pdf-file/1311818/NationalSemiconductor/NS16450/1>
use super::CharDevice;
use crate::sync::{Condvar, UPIntrFreeCell};
use crate::task::schedule;
use alloc::collections::VecDeque;
use bitflags::*;
use core::ops::Add;
use core::ptr::NonNull;
use volatile::access::*;
use volatile::VolatileRef;

bitflags! {
    /// InterruptEnableRegister
    #[derive(Clone, Copy)]
    pub struct IER: u8 {
        const RX_AVAILABLE = 1 << 0;
        const TX_EMPTY = 1 << 1;
    }

    /// LineStatusRegister
    #[derive(Clone, Copy)]
    pub struct LSR: u8 {
        const DATA_AVAILABLE = 1 << 0;
        const THR_EMPTY = 1 << 5;
    }

    /// Model Control Register
    #[derive(Clone, Copy)]
    pub struct MCR: u8 {
        const DATA_TERMINAL_READY = 1 << 0;
        const REQUEST_TO_SEND = 1 << 1;
        const AUX_OUTPUT1 = 1 << 2;
        const AUX_OUTPUT2 = 1 << 3;
    }
}

#[repr(C)]
struct ReadWithoutDLAB {
    /// receiver buffer register
    pub rbr: VolatileRef<'static, u8, ReadOnly>,
    /// interrupt enable register
    pub ier: VolatileRef<'static, IER, ReadWrite>,
    /// interrupt identification register
    pub iir: VolatileRef<'static, u8, ReadOnly>,
    /// line control register
    pub lcr: VolatileRef<'static, u8, ReadWrite>,
    /// model control register
    pub mcr: VolatileRef<'static, MCR, ReadWrite>,
    /// line status register
    pub lsr: VolatileRef<'static, LSR, ReadOnly>,
    /// ignore MSR
    _padding1: VolatileRef<'static, u8, ReadOnly>,
    /// ignore SCR
    _padding2: VolatileRef<'static, u8, ReadOnly>,
}

#[repr(C)]
struct WriteWithoutDLAB {
    /// transmitter holding register
    pub thr: VolatileRef<'static, u8, WriteOnly>,
    /// interrupt enable register
    pub ier: VolatileRef<'static, IER, ReadWrite>,
    /// ignore FCR
    _padding0: VolatileRef<'static, u8, ReadOnly>,
    /// line control register
    pub lcr: VolatileRef<'static, u8, ReadWrite>,
    /// modem control register
    pub mcr: VolatileRef<'static, MCR, ReadWrite>,
    /// line status register
    pub lsr: VolatileRef<'static, LSR, ReadOnly>,
    /// ignore other registers
    _padding1: VolatileRef<'static, u16, ReadOnly>,
}

pub struct NS16550aRaw {
    read_end: ReadWithoutDLAB,
    write_end: WriteWithoutDLAB,
}

impl NS16550aRaw {
    pub fn new(base_addr: usize) -> Self {
        let read_end = unsafe {
            ReadWithoutDLAB {
                rbr: VolatileRef::new_read_only(
                    NonNull::new(base_addr as *mut u8).expect("Base address is null"),
                ),
                ier: VolatileRef::new(
                    NonNull::new(base_addr.add(1) as *mut IER).expect("IER address is null"),
                ),
                iir: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(2) as *mut u8).expect("IIR address is null"),
                ),
                lcr: VolatileRef::new(
                    NonNull::new(base_addr.add(3) as *mut u8).expect("LCR address is null"),
                ),
                mcr: VolatileRef::new(
                    NonNull::new(base_addr.add(4) as *mut MCR).expect("MCR address is null"),
                ),
                lsr: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(5) as *mut LSR).expect("LSR address is null"),
                ),
                _padding1: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(6) as *mut u8).expect("Padding1 address is null"),
                ),
                _padding2: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(7) as *mut u8).expect("Padding2 address is null"),
                ),
            }
        };

        let write_end = unsafe {
            WriteWithoutDLAB {
                thr: VolatileRef::new_restricted(
                    WriteOnly,
                    NonNull::new(base_addr as *mut u8).expect("THR address is null"),
                ),
                ier: VolatileRef::new(
                    NonNull::new(base_addr.add(1) as *mut IER).expect("IER address is null"),
                ),
                _padding0: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(2) as *mut u8).expect("Padding0 address is null"),
                ),
                lcr: VolatileRef::new(
                    NonNull::new(base_addr.add(3) as *mut u8).expect("LCR address is null"),
                ),
                mcr: VolatileRef::new(
                    NonNull::new(base_addr.add(4) as *mut MCR).expect("MCR address is null"),
                ),
                lsr: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(5) as *mut LSR).expect("LSR address is null"),
                ),
                _padding1: VolatileRef::new_read_only(
                    NonNull::new(base_addr.add(6) as *mut u16).expect("Padding1 address is null"),
                ),
            }
        };

        Self {
            read_end,
            write_end,
        }
    }

    fn read_end(&mut self) -> &mut ReadWithoutDLAB {
        &mut self.read_end
    }

    fn write_end(&mut self) -> &mut WriteWithoutDLAB {
        &mut self.write_end
    }

    pub fn init(&mut self) {
        let read_end = self.read_end();
        let mut mcr = MCR::empty();
        mcr |= MCR::DATA_TERMINAL_READY;
        mcr |= MCR::REQUEST_TO_SEND;
        mcr |= MCR::AUX_OUTPUT2;
        read_end.mcr.as_mut_ptr().write(mcr);
        let ier = IER::RX_AVAILABLE;
        read_end.ier.as_mut_ptr().write(ier);
    }

    pub fn read(&mut self) -> Option<u8> {
        let read_end = self.read_end();
        let lsr = read_end.lsr.as_ptr().read();
        if lsr.contains(LSR::DATA_AVAILABLE) {
            Some(read_end.rbr.as_ptr().read())
        } else {
            None
        }
    }

    pub fn write(&mut self, ch: u8) {
        let write_end = self.write_end();
        loop {
            if write_end.lsr.as_ptr().read().contains(LSR::THR_EMPTY) {
                write_end.thr.as_mut_ptr().write(ch);
                break;
            }
        }
    }
}

struct NS16550aInner {
    ns16550a: NS16550aRaw,
    read_buffer: VecDeque<u8>,
}

pub struct NS16550a<const BASE_ADDR: usize> {
    inner: UPIntrFreeCell<NS16550aInner>,
    condvar: Condvar,
}

impl<const BASE_ADDR: usize> NS16550a<BASE_ADDR> {
    pub fn new() -> Self {
        let inner = NS16550aInner {
            ns16550a: NS16550aRaw::new(BASE_ADDR),
            read_buffer: VecDeque::new(),
        };
        Self {
            inner: unsafe { UPIntrFreeCell::new(inner) },
            condvar: Condvar::new(),
        }
    }

    #[allow(unused)]
    pub fn is_read_buffer_empty(&self) -> bool {
        self.inner
            .exclusive_session(|inner| inner.read_buffer.is_empty())
    }
}

impl<const BASE_ADDR: usize> CharDevice for NS16550a<BASE_ADDR> {
    fn init(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.ns16550a.init();
        drop(inner);
    }

    fn read(&self) -> u8 {
        loop {
            let mut inner = self.inner.exclusive_access();
            if let Some(ch) = inner.read_buffer.pop_front() {
                return ch;
            } else {
                let task_cx_ptr = self.condvar.wait_no_sched();
                drop(inner);
                schedule(task_cx_ptr);
            }
        }
    }
    fn write(&self, ch: u8) {
        let mut inner = self.inner.exclusive_access();
        inner.ns16550a.write(ch);
    }
    fn handle_irq(&self) {
        let mut count = 0;
        self.inner.exclusive_session(|inner| {
            while let Some(ch) = inner.ns16550a.read() {
                count += 1;
                inner.read_buffer.push_back(ch);
            }
        });
        if count > 0 {
            self.condvar.signal();
        }
    }
}
