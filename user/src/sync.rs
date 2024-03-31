use crate::syscall::{
    sys_condvar_create, sys_condvar_signal, sys_condvar_wait, sys_mutex_create, sys_mutex_lock,
    sys_mutex_unlock, sys_semaphore_create, sys_semaphore_down, sys_semaphore_up, sys_sleep,
};

pub fn sleep(sleep_ms: usize) {
    sys_sleep(sleep_ms);
}

pub fn mutex_create() -> isize {
    sys_mutex_create(false)
}

pub fn mutex_blocking_create() -> isize {
    sys_mutex_create(true)
}

pub fn mutex_lock(mutex_id: usize) {
    sys_mutex_lock(mutex_id);
}

pub fn mutex_unlock(mutex_id: usize) {
    sys_mutex_unlock(mutex_id);
}

pub fn semaphore_create(res_count: usize) -> isize {
    sys_semaphore_create(res_count)
}

pub fn semaphore_up(sem_id: usize) {
    sys_semaphore_up(sem_id);
}

pub fn semaphore_down(sem_id: usize) {
    sys_semaphore_down(sem_id);
}

pub fn condvar_create() -> isize {
    sys_condvar_create()
}

pub fn condvar_signal(condvar_id: usize) {
    sys_condvar_signal(condvar_id);
}

pub fn condvar_wait(condvar_id: usize, mutex_id: usize) {
    sys_condvar_wait(condvar_id, mutex_id);
}
