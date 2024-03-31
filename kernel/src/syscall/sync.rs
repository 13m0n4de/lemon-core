//! Synchronization Primitives System Calls

use alloc::sync::Arc;

use crate::{
    sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore},
    task::{block_current_and_run_next, current_process, current_task},
    timer::{add_timer, get_time_ms},
};

/// Puts the current task to sleep for a specified duration.
///
/// The function calculates the expiration time based on the current system time and
/// the specified duration, then adds a timer for the current task. The task is
/// blocked and other tasks are run until the timer expires, at which point the task
/// is unblocked.
///
/// # Arguments
///
/// - `ms`: The duration in milliseconds for which the current task should sleep.
///
/// # Returns
///
/// Always returns `0` to indicate successful sleep operation.
pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

/// Creates a new mutex.
///
/// Depending on the `blocking` flag, this function creates either a spin-lock mutex
/// (non-blocking) or a blocking mutex, and adds it to the current process's mutex list.
///
/// # Arguments
///
/// - `blocking`: A boolean flag indicating whether the mutex should be a blocking
/// mutex (`true`) or a spin-lock mutex (`false`).
///
/// # Returns
///
/// The index of the newly created mutex in the mutex list, which serves as its identifier.
pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let mutex: Option<Arc<dyn Mutex>> = if blocking {
        Some(Arc::new(MutexBlocking::new()))
    } else {
        Some(Arc::new(MutexSpin::new()))
    };

    if let Some(idx) = process_inner
        .mutex_list
        .iter()
        .position(core::option::Option::is_none)
    {
        process_inner.mutex_list[idx] = mutex;
        idx as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

/// Locks a specified mutex.
///
/// Attempts to lock the mutex identified by `mutex_id`. If the mutex is already locked,
/// the calling task will be blocked until the mutex becomes available (for blocking
/// mutexes) or it will spin (for spin-lock mutexes).
///
/// # Arguments
///
/// - `mutex_id`: The identifier of the mutex to lock, which corresponds to its index
/// in the current process's mutex list.
///
/// # Returns
///
/// - `0` on successful lock operation.
/// - `-1` if the mutex does not exist.
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.mutex_list.get(mutex_id) {
        Some(Some(mutex)) => {
            let mutex = Arc::clone(mutex);
            drop(process_inner);
            drop(process);
            mutex.lock();
            0
        }
        _ => -1,
    }
}

/// Unlocks a specified mutex.
///
/// Unlocks the mutex identified by `mutex_id`, potentially unblocking a task that is
/// waiting for this mutex.
///
/// # Arguments
///
/// - `mutex_id`: The identifier of the mutex to unlock, which corresponds to its
/// index in the current process's mutex list.
///
/// # Returns
///
/// - `0` on successful unlock operation.
/// - `-1` if the mutex does not exist.
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.mutex_list.get(mutex_id) {
        Some(Some(mutex)) => {
            let mutex = Arc::clone(mutex);
            drop(process_inner);
            drop(process);
            mutex.unlock();
            0
        }
        _ => -1,
    }
}

/// Creates a new semaphore with a specified resource count.
///
/// Initializes a semaphore with a given count of resources and adds it to the current
/// process's semaphore list.
///
/// # Arguments
///
/// - `res_count`: The initial resource count for the semaphore.
///
/// # Returns
///
/// The index of the newly created semaphore in the semaphore list, which serves as its
/// identifier.
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let semaphore = Some(Arc::new(Semaphore::new(res_count)));

    if let Some(idx) = process_inner
        .semaphore_list
        .iter()
        .position(core::option::Option::is_none)
    {
        process_inner.semaphore_list[idx] = semaphore;
        idx as isize
    } else {
        process_inner.semaphore_list.push(semaphore);
        process_inner.semaphore_list.len() as isize - 1
    }
}

/// Increments the resource count of a specified semaphore.
///
/// Signals (increments) the semaphore identified by `sem_id`, potentially unblocking
/// tasks that are waiting for resources.
///
/// # Arguments
///
/// - `sem_id`: The identifier of the semaphore to signal, which corresponds to its
/// index in the current process's semaphore list.
///
/// # Returns
///
/// - `0` on successful operation.
/// - `-1` if the semaphore does not exist.
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.semaphore_list.get(sem_id) {
        Some(Some(semaphore)) => {
            let semaphore = Arc::clone(semaphore);
            drop(process_inner);
            drop(process);
            semaphore.up();
            0
        }
        _ => -1,
    }
}

/// Decrements the resource count of a specified semaphore.
///
/// Waits (decrements) the semaphore identified by `sem_id`. If the semaphore's resource
/// count is zero, the calling task will block until a resource becomes available.
///
/// # Arguments
///
/// - `sem_id`: The identifier of the semaphore to wait on, which corresponds to its
/// index in the current process's semaphore list.
///
/// # Returns
///
/// - `0` on successful operation.
/// - `-1` if the semaphore does not exist.
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.semaphore_list.get(sem_id) {
        Some(Some(semaphore)) => {
            let semaphore = Arc::clone(semaphore);
            drop(process_inner);
            drop(process);
            semaphore.down();
            0
        }
        _ => -1,
    }
}

/// Creates a new condition variable.
///
/// Adds a new condition variable to the current process's condition variable list.
///
/// # Returns
///
/// The index of the newly created condition variable in the condition variable list,
/// which serves as its identifier.
pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let condvar = Some(Arc::new(Condvar::new()));

    if let Some(idx) = process_inner
        .condvar_list
        .iter()
        .position(core::option::Option::is_none)
    {
        process_inner.condvar_list[idx] = condvar;
        idx as isize
    } else {
        process_inner.condvar_list.push(condvar);
        process_inner.condvar_list.len() as isize - 1
    }
}

/// Signals a specified condition variable.
///
/// Wakes up one task waiting on the condition variable identified by `condvar_id`.
///
/// # Arguments
///
/// - `condvar_id`: The identifier of the condition variable to signal, which corresponds
/// to its index in the current process's condition variable list.
///
/// # Returns
///
/// - `0` on successful operation.
/// - `-1` if the condition variable does not exist.
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.condvar_list.get(condvar_id) {
        Some(Some(condvar)) => {
            let condvar = Arc::clone(condvar);
            drop(process_inner);
            drop(process);
            condvar.signal();
            0
        }
        _ => -1,
    }
}

/// Waits on a specified condition variable.
///
/// Blocks the calling task until the condition variable identified by `condvar_id` is
/// signaled. The task automatically re-acquires the mutex identified by `mutex_id`
/// upon waking up.
///
/// # Arguments
///
/// - `condvar_id`: The identifier of the condition variable to wait on, which corresponds
/// to its index in the current process's condition variable list.
/// - `mutex_id`: The identifier of the mutex to be released while waiting and
/// re-acquired upon waking up, which corresponds to its index in the current
/// process's mutex list.
///
/// # Returns
///
/// - `0` on successful operation.
/// - `-1` if either the condition variable or the mutex does not exist, or if any other
/// error occurs.
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    match process_inner.condvar_list.get(condvar_id) {
        Some(Some(condvar)) => match process_inner.mutex_list.get(mutex_id) {
            Some(Some(mutex)) => {
                let mutex = Arc::clone(mutex);
                let condvar = Arc::clone(condvar);
                drop(process_inner);
                drop(process);
                condvar.wait_with_mutex(mutex);
                0
            }
            _ => -1,
        },
        _ => -1,
    }
}
