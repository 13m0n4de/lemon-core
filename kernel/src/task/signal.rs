use super::current_pcb;
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SignalFlags: u32 {
        const SIGINT    = 1 << 2;
        const SIGILL    = 1 << 4;
        const SIGABRT   = 1 << 6;
        const SIGFPE    = 1 << 8;
        const SIGSEGV   = 1 << 11;
    }
}

impl SignalFlags {
    pub fn check_error(self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "Killed, SIGINT=2"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "Illegal Instruction, SIGILL=4"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "Aborted, SIGABRT=6"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            None
        }
    }
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let process = current_pcb();
    let process_inner = process.inner_exclusive_access();
    process_inner.signals.check_error()
}

pub fn add_signal_to_current(signal: SignalFlags) {
    let process = current_pcb();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.signals |= signal;
}
