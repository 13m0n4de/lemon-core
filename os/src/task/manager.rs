//! Implementation of `TaskManager`

use crate::sync::UPSafeCell;

use super::tcb::TaskControlBlock;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;

/// A array of [`TaskControlBlock`] that is thread-safe
pub struct Manager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler
impl Manager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    /// Add a task to [`Manager`]
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// Remove the first task and return it,or [`None`] if [`Manager`] is empty
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    static ref TASK_MANAGER: UPSafeCell<Manager> = unsafe { UPSafeCell::new(Manager::new()) };
}

/// Interface offered to add task
pub fn add(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// Interface offered to pop the first task
pub fn fetch() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
