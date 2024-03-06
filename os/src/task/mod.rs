//! # Task Management
//!
//! Use [`TaskManager`] to manage tasks, and use [`__switch`] to switch tasks.

mod context;
mod manager;
mod switch;

use crate::config::{kernel_stack_position, TRAP_CONTEXT};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE};
use crate::trap::{trap_handler, TrapContext};

use context::TaskContext;
use switch::__switch;

pub use manager::{
    current_trap_cx, current_user_token, exit_current_and_run_next, run_first_task,
    suspend_current_and_run_next, user_time_end, user_time_start,
};

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
