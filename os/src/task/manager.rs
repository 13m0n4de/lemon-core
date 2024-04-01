//! Implementation of [`TaskManager`]

use crate::sync::UPSafeCell;

use super::tcb::TaskControlBlock;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::lazy_static;

/// A array of `TaskControlBlock` that is thread-safe
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

    /// Add a task to [`TaskManager`]
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    /// Remove the first task and return it,or [`None`] if [`TaskManager`] is empty
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

lazy_static! {
    static ref TASK_MANAGER: UPSafeCell<Manager> = unsafe { UPSafeCell::new(Manager::new()) };
    static ref PID2TCB: UPSafeCell<BTreeMap<usize, Arc<TaskControlBlock>>> =
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// Interface offered to add task
pub fn add(task: Arc<TaskControlBlock>) {
    PID2TCB
        .exclusive_access()
        .insert(task.getpid(), task.clone());
    TASK_MANAGER.exclusive_access().add(task);
}

/// Interface offered to pop the first task
pub fn fetch() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn pid2task(pid: usize) -> Option<Arc<TaskControlBlock>> {
    let map = PID2TCB.exclusive_access();
    map.get(&pid).cloned()
}

pub fn remove_from_pid2task(pid: usize) {
    let mut map = PID2TCB.exclusive_access();
    assert!(
        map.remove(&pid).is_some(),
        "cannot find pid {pid} in pid2task!"
    );
}
