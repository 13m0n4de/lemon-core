use super::UPIntrFreeCell;
use crate::task::{
    block_current_and_run_next, current_tcb, manager, suspend_current_and_run_next,
    tcb::TaskControlBlock,
};
use alloc::{collections::VecDeque, sync::Arc};

/// A trait for Mutex mechanisms, ensuring thread safety.
pub trait Mutex: Sync + Send {
    /// Locks the mutex, blocking the current thread until it becomes available.
    fn lock(&self);
    /// Unlocks the mutex, allowing other threads to acquire it.
    fn unlock(&self);
}

/// A spinning mutex implementation.
pub struct Spin {
    locked: UPIntrFreeCell<bool>,
}

impl Spin {
    /// Creates a new, unlocked spinning mutex.
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPIntrFreeCell::new(false) },
        }
    }
}

impl Mutex for Spin {
    /// Locks the mutex using a spin-wait loop.
    fn lock(&self) {
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            }
            *locked = true;
            break;
        }
    }

    /// Unlocks the mutex, making it available for other threads.
    fn unlock(&self) {
        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// A blocking mutex implementation.
pub struct Blocking {
    inner: UPIntrFreeCell<BlockingInner>,
}

pub struct BlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Blocking {
    /// Creates a new, unlocked blocking mutex.
    pub fn new() -> Self {
        Self {
            inner: unsafe {
                UPIntrFreeCell::new(BlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for Blocking {
    /// Locks the mutex, blocking the current thread if the mutex is already locked.
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_tcb().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// Unlocks the mutex, waking up the next task in the waiting queue if any.
    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            manager::wakeup(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
