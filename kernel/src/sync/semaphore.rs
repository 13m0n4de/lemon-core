use alloc::{collections::VecDeque, sync::Arc};

use crate::task::{block_current_and_run_next, current_tcb, manager, tcb::TaskControlBlock};

use super::UPIntrFreeCell;

pub struct Semaphore {
    pub inner: UPIntrFreeCell<Inner>,
}

pub struct Inner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    pub fn new(res_count: usize) -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(Inner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    pub fn up(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                manager::wakeup(task);
            }
        }
    }

    pub fn down(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_tcb().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
