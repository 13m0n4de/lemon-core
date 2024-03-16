//! Implementation of [`TaskManager`]

use crate::sync::UPSafeCell;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::lazy_static;

use super::pcb::ProcessControlBlock;
use super::tcb::TaskControlBlock;

/// A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler
impl TaskManager {
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

    pub fn remove(&mut self, task: Arc<TaskControlBlock>) {
        if let Some(idx) = self
            .ready_queue
            .iter()
            .position(|t| Arc::as_ptr(t) == Arc::as_ptr(&task))
        {
            self.ready_queue.remove(idx);
        }
    }
}

lazy_static! {
    static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
    static ref PID2PCB: UPSafeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

/// Add the thread to the ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

/// Pop a task from the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

/// Remove the task from the ready queue
pub fn remove_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().remove(task);
}

/// Query the PCB based on PID
pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.exclusive_access();
    map.get(&pid).cloned()
}

/// Remove the PCB based on PID
pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    if map.remove(&pid).is_none() {
        panic!("cannot find pid {} in pid2task!", pid);
    }
}

/// Add a pair of PID-PCB mappings
pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}
