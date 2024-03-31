//! RISC-V timer-related functionality

use core::cmp::Ordering;

use crate::config::CLOCK_FREQ;
use crate::sbi::set_timer;
use crate::sync::UPIntrFreeCell;
use crate::task::{wakeup, ControlBlock};
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

/// read the `mtime` register
pub fn get_time() -> usize {
    time::read()
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// set the next timer interrupt
pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}

pub struct CondVar {
    pub expire_ms: usize,
    pub task: Arc<ControlBlock>,
}

impl PartialEq for CondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire_ms == other.expire_ms
    }
}

impl Eq for CondVar {}

impl PartialOrd for CondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.expire_ms.cmp(&self.expire_ms))
    }
}

impl Ord for CondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static! {
    static ref TIMERS: UPIntrFreeCell<BinaryHeap<CondVar>> =
        unsafe { UPIntrFreeCell::new(BinaryHeap::<CondVar>::new()) };
}

pub fn add(expire_ms: usize, task: Arc<ControlBlock>) {
    let mut timers = TIMERS.exclusive_access();
    timers.push(CondVar { expire_ms, task });
}

pub fn check() {
    let current_ms = get_time_ms();
    let mut timers = TIMERS.exclusive_access();
    while let Some(timer) = timers.peek() {
        if timer.expire_ms <= current_ms {
            wakeup(timer.task.clone());
            timers.pop();
        } else {
            break;
        }
    }
}
