//! Implementation of `TaskContext`

use crate::trap::leave;

/// Task Context
#[repr(C)]
pub struct Context {
    // return address ( e.g. __restore ) of __switch ASM function
    ra: usize,
    // kernel stack pointer of app
    sp: usize,
    // callee saved registers:  s 0..11
    s: [usize; 12],
}

impl Context {
    /// Init task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    /// Set Task Context{__restore ASM funciton: `trap_return`, sp: `kstack_ptr`, s: `s_0..12`}
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: leave as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
