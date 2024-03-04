//! # Task Management
//!
//! Use [`TaskManager`] to manage tasks, and use [`__switch`] to switch tasks.

mod context;
mod manager;
mod switch;

use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::loader::{get_app_data, get_num_app};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::sync::UPSafeCell;
use crate::timer::get_time_ms;
use crate::trap::{trap_handler, TrapContext};
use alloc::vec::Vec;
use lazy_static::*;
use log::*;
use switch::__switch;

use context::TaskContext;
use manager::TaskManager;

struct TaskPool {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
    stop_watch: usize,
}

struct TaskControlBlock {
    task_status: TaskStatus,
    task_cx: TaskContext,
    memory_set: MemorySet,
    trap_cx_ppn: PhysPageNum,
    #[allow(dead_code)]
    base_size: usize,
    user_time: usize,
    kernel_time: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;

        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            kernel_time: 0,
            user_time: 0,
        };

        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }

    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
}

#[derive(Copy, Clone, PartialEq)]
enum TaskStatus {
    Ready,
    Running,
    Exited,
}

impl TaskPool {
    fn refresh_stop_watch(&mut self) -> usize {
        let start_time = self.stop_watch;
        self.stop_watch = get_time_ms();
        self.stop_watch - start_time
    }
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        debug!("num_app = {num_app}");

        let tasks: Vec<TaskControlBlock> = (0..num_app)
            .map(|i| TaskControlBlock::new(get_app_data(i), i))
            .collect();

        let inner = unsafe {
            UPSafeCell::new(TaskPool {
                tasks,
                current_task: 0,
                stop_watch: 0,
            })
        };
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

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

// Counting kernel time, starting from now is user time.
pub fn user_time_start() {
    TASK_MANAGER.user_time_start()
}

// Counting user time, starting from now is kernel time.
pub fn user_time_end() {
    TASK_MANAGER.user_time_end()
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}
