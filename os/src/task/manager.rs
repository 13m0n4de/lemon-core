//! Implementation of `TaskManager`

use super::{Context as TaskContext, TaskControlBlock, TaskStatus, __switch};
use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::trap::Context as TrapContext;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::{debug, info, trace};

pub struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
    stop_watch: usize,
}

impl TaskManagerInner {
    fn refresh_stop_watch(&mut self) -> usize {
        let start_time = self.stop_watch;
        self.stop_watch = get_time_ms();
        self.stop_watch - start_time
    }
}

pub struct Manager {
    pub num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

impl Manager {
    // Run the first task in task list.
    pub fn run_first_task(&self) -> ! {
        let mut pool = self.inner.exclusive_access();
        let task0 = &mut pool.tasks[0];

        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;

        pool.refresh_stop_watch();

        drop(pool);
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(
                core::ptr::from_mut(&mut TaskContext::zero_init()),
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }

    // Find next task to run and return app id.
    pub fn mark_current_exited(&self) {
        let mut pool = self.inner.exclusive_access();
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
        let pool = self.inner.exclusive_access();
        let current = pool.current_task;
        ((current + 1)..(current + self.num_app))
            .map(|id| id % self.num_app)
            .find(|id| pool.tasks[*id].task_status == TaskStatus::Ready)
    }

    // Change the status of current `Running` task into `Ready`.
    pub fn mark_current_suspended(&self) {
        let mut pool = self.inner.exclusive_access();
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
            let mut pool = self.inner.exclusive_access();
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
        let mut pool = self.inner.exclusive_access();
        let current = pool.current_task;
        pool.tasks[current].kernel_time += pool.refresh_stop_watch();
    }

    // Counting user time, starting from now is kernel time.
    pub fn user_time_end(&self) {
        let mut pool = self.inner.exclusive_access();
        let current = pool.current_task;
        pool.tasks[current].user_time += pool.refresh_stop_watch();
    }

    /// Get the current 'Running' task's token.
    pub fn current_token(&self) -> usize {
        let pool = self.inner.exclusive_access();
        pool.tasks[pool.current_task].user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    pub fn current_trap_cx(&self) -> &'static mut TrapContext {
        let pool = self.inner.exclusive_access();
        pool.tasks[pool.current_task].trap_cx()
    }
}

lazy_static! {
    static ref TASK_MANAGER: Manager = {
        let num_app = get_num_app();
        debug!("num_app = {num_app}");

        let tasks: Vec<TaskControlBlock> = (0..num_app)
            .map(|i| TaskControlBlock::new(get_app_data(i), i))
            .collect();

        let inner = unsafe {
            UPSafeCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
                stop_watch: 0,
            })
        };
        Manager { num_app, inner }
    };
}

/// Run first task
pub fn run_first() {
    TASK_MANAGER.run_first_task();
}

/// Exit current task,  then run next task
pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}

/// Suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

// Counting kernel time, starting from now is user time.
pub fn user_time_start() {
    TASK_MANAGER.user_time_start();
}

// Counting user time, starting from now is kernel time.
pub fn user_time_end() {
    TASK_MANAGER.user_time_end();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.current_trap_cx()
}
