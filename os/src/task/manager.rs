//! Implementation of [`TaskManager`]

use super::{TaskContext, TaskPool, TaskStatus, __switch};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use log::*;

pub struct TaskManager {
    pub num_app: usize,
    pub pool: UPSafeCell<TaskPool>,
}

impl TaskManager {
    // Run the first task in task list.
    pub fn run_first_task(&self) -> ! {
        let mut pool = self.pool.exclusive_access();
        let task0 = &mut pool.tasks[0];

        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        pool.refresh_stop_watch();

        drop(pool);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    // Find next task to run and return app id.
    pub fn mark_current_exited(&self) {
        let mut pool = self.pool.exclusive_access();
        let current = pool.current_task;

        pool.tasks[current].kernel_time += pool.refresh_stop_watch();
        trace!(
            "[kernel] Task {} exited. user_time: {} ms, kernle_time: {} ms.",
            current,
            pool.tasks[current].user_time,
            pool.tasks[current].kernel_time
        );
        pool.tasks[current].task_status = TaskStatus::Exited;
    }

    // Find next task to run and return app id.
    fn find_next_task(&self) -> Option<usize> {
        let pool = self.pool.exclusive_access();
        let current = pool.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| pool.tasks[*id].task_status == TaskStatus::Ready)
    }

    // Change the status of current `Running` task into `Ready`.
    pub fn mark_current_suspended(&self) {
        let mut pool = self.pool.exclusive_access();
        let current = pool.current_task;

        trace!("[kernel] Task {} suspended", current);

        pool.tasks[current].kernel_time += pool.refresh_stop_watch();
        pool.tasks[current].task_status = TaskStatus::Ready;
    }

    // Find the next 'Ready' task and set its status to 'Running'.
    // Update `current_task` to this task.
    // Call `__switch` to switch tasks.
    pub fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut pool = self.pool.exclusive_access();
            let current = pool.current_task;
            trace!("[kernel] Task {} start", current);
            pool.tasks[next].task_status = TaskStatus::Running;
            pool.current_task = next;
            let current_task_cx_ptr = &mut pool.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &pool.tasks[next].task_cx as *const TaskContext;
            drop(pool);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            info!("[kernel] All applications completed!");
            shutdown(false);
        }
    }

    // Counting kernel time, starting from now is user time.
    pub fn user_time_start(&self) {
        let mut pool = self.pool.exclusive_access();
        let current = pool.current_task;
        pool.tasks[current].kernel_time += pool.refresh_stop_watch();
    }

    // Counting user time, starting from now is kernel time.
    pub fn user_time_end(&self) {
        let mut pool = self.pool.exclusive_access();
        let current = pool.current_task;
        pool.tasks[current].user_time += pool.refresh_stop_watch();
    }

    /// Get the current 'Running' task's token.
    pub fn get_current_token(&self) -> usize {
        let pool = self.pool.exclusive_access();
        pool.tasks[pool.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    pub fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let pool = self.pool.exclusive_access();
        pool.tasks[pool.current_task].get_trap_cx()
    }
}
