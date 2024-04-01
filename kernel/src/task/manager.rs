//! Implementation of `TaskManager`

use crate::sync::UPIntrFreeCell;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use lazy_static::lazy_static;

use super::pcb::ProcessControlBlock;
use super::tcb::{Status, TaskControlBlock};

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
    static ref TASK_MANAGER: UPIntrFreeCell<Manager> =
        unsafe { UPIntrFreeCell::new(Manager::new()) };
    static ref PID2PCB: UPIntrFreeCell<BTreeMap<usize, Arc<ProcessControlBlock>>> =
        unsafe { UPIntrFreeCell::new(BTreeMap::new()) };
}

/// Add the thread to the ready queue
pub fn add(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn wakeup(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = Status::Ready;
    drop(task_inner);
    add(task);
}

/// Pop a task from the ready queue
pub fn fetch() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

/// Query the PCB based on PID
pub fn pid2process(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    let map = PID2PCB.exclusive_access();
    map.get(&pid).cloned()
}

/// Remove the PCB based on PID
pub fn remove_from_pid2process(pid: usize) {
    let mut map = PID2PCB.exclusive_access();
    assert!(
        map.remove(&pid).is_some(),
        "cannot find pid {pid} in pid2task!"
    );
}

/// Add a pair of PID-PCB mappings
pub fn insert_into_pid2process(pid: usize, process: Arc<ProcessControlBlock>) {
    PID2PCB.exclusive_access().insert(pid, process);
}
