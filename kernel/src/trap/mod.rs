//! Trap Handling Module
//!
//! - Set CSR `stvec` to `__alltraps`.
//! - On trap, system jumps to `__alltraps`.
//!   - saves context.
//!   - switches stack form user to kernel.
//!   - call [`trap_handler`]
//! - Handle [`Exception`] and [`Interrupt`]

mod context;

use crate::{
    config::TRAMPOLINE,
    syscall::syscall,
    task::{
        add_signal_to_current, check_signals_error_of_current, current_trap_cx,
        current_trap_cx_user_va, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, SignalFlags,
    },
    timer::{check_timer, set_next_trigger},
};
use core::arch::{asm, global_asm};
use log::debug;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

/// handle an interrupt, exception, or system call from user space
#[no_mangle]
pub extern "C" fn user_handler() -> ! {
    set_kernel_trap_entry();
    let mut cx = current_trap_cx();
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value

    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // jump to next instruction anyway
            cx.sepc += 4;
            // get system call return value
            let result = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]);
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.x[10] = result as usize;
        }
        Trap::Exception(
            Exception::StoreFault
            | Exception::StorePageFault
            | Exception::InstructionFault
            | Exception::InstructionPageFault
            | Exception::LoadFault
            | Exception::LoadPageFault,
        ) => {
            debug!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                scause.cause(),
                stval,
                cx.sepc,
            );
            add_signal_to_current(SignalFlags::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            debug!("[kernel] IllegalInstruction in application, kernel killed it.");
            add_signal_to_current(SignalFlags::SIGILL);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            check_timer();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    // check signals
    if let Some((errno, msg)) = check_signals_error_of_current() {
        debug!("[kernel] {}", msg);
        exit_current_and_run_next(errno);
    }

    leave()
}

/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = `trap_cx_ptr`, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
#[no_mangle]
pub extern "C" fn leave() -> ! {
    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    set_user_trap_entry();
    let trap_cx_user_va = current_trap_cx_user_va();
    let user_satp = current_user_token();
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_user_va,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        )
    }
}

#[no_mangle]
/// Unimplement: traps/interrupts/exceptions from kernel mode
/// Todo: Chapter 9: I/O device
pub extern "C" fn kernel_handler() -> ! {
    use riscv::register::sepc;
    debug!("stval = {:#x}, sepc = {:#x}", stval::read(), sepc::read());
    panic!("a trap {:?} from kernel!", scause::read().cause());
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(kernel_handler as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

pub use context::Context;
