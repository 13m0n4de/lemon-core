use bitflags::bitflags;
use log::debug;

use super::{current_tcb, suspend_current_and_run_next};

pub const MAX_SIG: usize = 31;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct SignalFlags: u32 {
        const SIGDEF = 1 << 0;
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGBUS = 1 << 7;
        const SIGFPE = 1 << 8;
        const SIGKILL = 1 << 9;
        const SIGUSR1 = 1 << 10;
        const SIGSEGV = 1 << 11;
        const SIGUSR2 = 1 << 12;
        const SIGPIPE = 1 << 13;
        const SIGALRM = 1 << 14;
        const SIGTERM = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD = 1 << 17;
        const SIGCONT = 1 << 18;
        const SIGSTOP = 1 << 19;
        const SIGTSTP = 1 << 20;
        const SIGTTIN = 1 << 21;
        const SIGTTOU = 1 << 22;
        const SIGURG = 1 << 23;
        const SIGXCPU = 1 << 24;
        const SIGXFSZ = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF = 1 << 27;
        const SIGWINCH = 1 << 28;
        const SIGIO = 1 << 29;
        const SIGPWR = 1 << 30;
        const SIGSYS = 1 << 31;
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
        } else if self.contains(Self::SIGKILL) {
            Some((-9, "Killed, SIGKILL=9"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            None
        }
    }
}

/// Action for a signal
#[derive(Clone, Copy)]
#[repr(C, align(16))]
#[allow(clippy::module_name_repetitions)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

impl Default for SignalAction {
    fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::from_bits(40).unwrap(),
        }
    }
}

#[derive(Clone, Copy)]
#[allow(clippy::module_name_repetitions)]
pub struct SignalActions {
    pub table: [SignalAction; MAX_SIG + 1],
}

impl Default for SignalActions {
    fn default() -> Self {
        Self {
            table: [SignalAction::default(); MAX_SIG + 1],
        }
    }
}

pub fn add_signal_to_current(signal: SignalFlags) {
    let task = current_tcb().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
}

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let task = current_tcb().unwrap();
        let task_inner = task.inner_exclusive_access();
        if !task_inner.frozen || task_inner.killed {
            break;
        }
        suspend_current_and_run_next();
    }
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_tcb().unwrap();
    let task_inner = task.inner_exclusive_access();
    task_inner.signals.check_error()
}

fn check_pending_signals() {
    for sig in 0..=MAX_SIG {
        let task = current_tcb().unwrap();
        let task_inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {
            let masked = matches!(
                task_inner.handling_sig,
                Some(sig) if task_inner.signal_actions.table[sig].mask.contains(signal)
            );

            if !masked {
                drop(task_inner);
                drop(task);
                if matches!(
                    signal,
                    SignalFlags::SIGILL
                        | SignalFlags::SIGSTOP
                        | SignalFlags::SIGCONT
                        | SignalFlags::SIGDEF
                ) {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    break;
                }
            }
        }
    }
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_tcb().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;
            task_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                task_inner.signals ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            task_inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_tcb().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let handler = task_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // user handler

        // handle flag
        task_inner.handling_sig = Some(sig);
        task_inner.signals ^= signal;

        // backup trapframe
        let trap_ctx = task_inner.trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // modify trapframe
        trap_ctx.sepc = handler;

        // put args (a0)
        trap_ctx.x[10] = sig;
    } else {
        // default action
        debug!(
            "[kernel] task::call_user_signal_handler: default action: ignore it or kill process"
        );
    }
}
