//! # Task Management
//!
//! Use [`TaskManager`] to manage tasks, and use [`__switch`] to switch tasks.

mod context;
mod switch;

use crate::config::MAX_APP_NUM;
use crate::loader::init_app_cx;
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::lazy_static;
use log::info;
use switch::__switch;

pub use context::Context;

struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

#[derive(Copy, Clone)]
struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: Context,
}

#[derive(Copy, Clone, PartialEq)]
enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

impl TaskManagerInner {
    fn new() -> Self {
        let mut tasks = [TaskControlBlock {
            task_cx: Context::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = Context::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        Self {
            tasks,
            current_task: 0,
        }
    }
}

impl TaskManager {
    // Run the first task in task list.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const Context;
        drop(inner);
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(
                core::ptr::from_mut::<Context>(&mut Context::zero_init()),
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }

    // Find next task to run and return app id.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    // Find next task to run and return app id.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        ((current + 1)..=(current + self.num_app))
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    // Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    // Find the next 'Ready' task and set its status to 'Running'.
    // Update `current_task` to this task.
    // Call `__switch` to switch tasks.
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut Context;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const Context;
            drop(inner);
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
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        extern "C" {
            fn _num_app();
        }
        let num_app = unsafe { (_num_app as usize as *const usize).read_volatile() };
        let inner = unsafe { UPSafeCell::new(TaskManagerInner::new()) };
        TaskManager { num_app, inner }
    };
}

/// run first task
pub fn run_first() {
    TASK_MANAGER.run_first_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}
