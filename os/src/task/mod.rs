mod context;
mod switch;

use crate::config::MAX_APP_NUM;
use crate::loader::init_app_cx;
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use lazy_static::*;
use log::*;
use switch::__switch;

pub use context::TaskContext;

struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskPool>,
}

struct TaskPool {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_task: usize,
}

#[derive(Copy, Clone)]
struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
}

#[derive(Copy, Clone, PartialEq)]
enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

impl TaskPool {
    fn new() -> Self {
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        Self {
            tasks,
            current_task: 0,
        }
    }
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    fn run_next_task(&self) -> ! {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            panic!("unreachable in run_first_task!");
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
        let inner = unsafe { UPSafeCell::new(TaskPool::new()) };
        TaskManager { num_app, inner }
    };
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
}
