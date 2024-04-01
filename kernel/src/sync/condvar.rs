use alloc::{collections::VecDeque, sync::Arc};

use crate::task::{block_current_and_run_next, current_tcb, wakeup_task, TaskControlBlock};

use super::{Mutex, UPSafeCell};

pub struct Condvar {
    pub inner: UPSafeCell<Inner>,
}

pub struct Inner {
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPSafeCell::new(Inner {
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn signal(&self) {
        let mut inner = self.inner.exclusive_access();
        if let Some(task) = inner.wait_queue.pop_front() {
            wakeup_task(task);
        }
    }

    pub fn wait(&self, mutex: &Arc<dyn Mutex>) {
        mutex.unlock();
        let mut inner = self.inner.exclusive_access();
        inner.wait_queue.push_back(current_tcb().unwrap());
        drop(inner);
        block_current_and_run_next();
        mutex.lock();
    }
}
