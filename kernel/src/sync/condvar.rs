use alloc::{collections::VecDeque, sync::Arc};

use crate::task;
use crate::task::{block_current, block_current_and_run_next, current_tcb};

use super::{up::UPIntrFreeCell, Mutex};

pub struct Condvar {
    pub inner: UPIntrFreeCell<Inner>,
}

pub struct Inner {
    pub wait_queue: VecDeque<Arc<task::ControlBlock>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(Inner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            task::wakeup(task);
        }
    }

    pub fn wait_with_mutex(&self, mutex: Arc<dyn Mutex>) {
        mutex.unlock();
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_tcb().unwrap());
        });
        block_current_and_run_next();
        mutex.lock();
    }

    pub fn wait_no_sched(&self) -> *mut task::Context {
        self.inner.exclusive_session(|inner| {
            inner.wait_queue.push_back(current_tcb().unwrap());
        });
        block_current()
    }
}
