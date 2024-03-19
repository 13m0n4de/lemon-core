use alloc::sync::Arc;

use crate::{
    sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore},
    task::{block_current_and_run_next, current_process, current_task},
    timer::{add_timer, get_time_ms},
};

pub fn sys_sleep(ms: usize) -> isize {
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}

pub fn sys_mutex_create(blocking: bool) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };

    if let Some(idx) = process_inner
        .mutex_list
        .iter()
        .position(|mutex| mutex.is_none())
    {
        process_inner.mutex_list[idx] = mutex;
        idx as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}

pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if let Some(mutex) = process_inner.mutex_list.get(mutex_id) {
        if let Some(mutex_ref) = mutex.clone().as_ref() {
            drop(process_inner);
            drop(process);
            mutex_ref.lock();
            return 0;
        }
    }
    -1
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if let Some(mutex) = process_inner.mutex_list.get(mutex_id) {
        if let Some(mutex_ref) = mutex.clone().as_ref() {
            drop(process_inner);
            drop(process);
            mutex_ref.unlock();
            return 0;
        }
    }
    -1
}

pub fn sys_semaphore_create(res_count: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let semaphore = Some(Arc::new(Semaphore::new(res_count)));

    if let Some(idx) = process_inner
        .semaphore_list
        .iter()
        .position(|semaphore| semaphore.is_none())
    {
        process_inner.semaphore_list[idx] = semaphore;
        idx as isize
    } else {
        process_inner.semaphore_list.push(semaphore);
        process_inner.semaphore_list.len() as isize - 1
    }
}

pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if let Some(semaphore) = process_inner.semaphore_list.get(sem_id) {
        if let Some(semaphore_ref) = semaphore.clone().as_ref() {
            drop(process_inner);
            drop(process);
            semaphore_ref.up();
            return 0;
        }
    }
    -1
}

pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if let Some(semaphore) = process_inner.semaphore_list.get(sem_id) {
        if let Some(semaphore_ref) = semaphore.clone().as_ref() {
            drop(process_inner);
            drop(process);
            semaphore_ref.down();
            return 0;
        }
    }
    -1
}

pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let condvar = Some(Arc::new(Condvar::new()));

    if let Some(idx) = process_inner
        .condvar_list
        .iter()
        .position(|condvar| condvar.is_none())
    {
        process_inner.condvar_list[idx] = condvar;
        idx as isize
    } else {
        process_inner.condvar_list.push(condvar);
        process_inner.condvar_list.len() as isize - 1
    }
}

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
                condvar.wait(mutex);
                0
            }
            _ => -1,
        },
        _ => -1,
    }
}
