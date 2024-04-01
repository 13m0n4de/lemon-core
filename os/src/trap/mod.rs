//! Trap Handling Module
//!
//! - Set CSR `stvec` to `__alltraps`.
//! - On trap, system jumps to `__alltraps`.
//!   - saves context.
//!   - switches stack form user to kernel.
//!   - call [`user_handler`]
//! - Handle syscall or other exceptions

mod context;

use crate::{syscall::syscall, task::exit_current_and_run_next};
use core::arch::global_asm;
use log::info;
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Trap},
    stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    extern "C" {
        fn __alltraps();
    }

    unsafe {
        stvec::write(__alltraps as usize, TrapMode::Direct);
    }
}

/// handle an interrupt, exception, or system call from user space
#[no_mangle]
pub extern "C" fn user_handler(cx: &mut Context) -> &mut Context {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault | Exception::StorePageFault) => {
            info!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            info!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    cx
}

pub use context::Context;
