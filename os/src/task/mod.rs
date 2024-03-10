//! # Task Management

mod context;
mod control_block;
mod manager;
mod pid;
mod processor;
mod signal;
mod switch;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use log::*;

use crate::{
    fs::{open_file, OpenFlags},
    sbi::shutdown,
};
use context::TaskContext;
use processor::{schedule, take_current_task};

pub use manager::{add_task, pid2task};
pub use processor::{current_task, current_trap_cx, current_user_token, run_tasks};
pub use signal::{SignalAction, SignalActions, SignalFlags, MAX_SIG};

use self::{control_block::TaskControlBlock, manager::remove_from_pid2task};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
}

lazy_static! {
    /// Global process that init user shell
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}

/// Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Suspend the current 'Running' task and run the next task in task list
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // --- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // --- release current TCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle.
    schedule(task_cx_ptr);
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        info!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        if exit_code != 0 {
            shutdown(true)
        } else {
            shutdown(false)
        }
    }

    // remove from pid2task
    remove_from_pid2task(task.getpid());
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

pub fn add_signal_to_current(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.signals |= signal;
}

pub fn handle_signals() {
    loop {
        check_pending_signals();
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        if !task_inner.frozen || task_inner.killed {
            break;
        }
        suspend_current_and_run_next();
    }
}

fn check_pending_signals() {
    for sig in 0..=MAX_SIG {
        let task = current_task().unwrap();
        let task_inner = task.inner_exclusive_access();
        let signal = SignalFlags::from_bits(1 << sig).unwrap();
        if task_inner.signals.contains(signal) && (!task_inner.signal_mask.contains(signal)) {
            let masked = match task_inner.handling_sig {
                Some(sig) if task_inner.signal_actions.table[sig].mask.contains(signal) => true,
                _ => false,
            };

            if !masked {
                drop(task_inner);
                drop(task);
                if matches!(
                    signal,
                    SignalFlags::SIGILL
                        | SignalFlags::SIGSTOP
                        | SignalFlags::SIGCONT
                        | SignalFlags::SIGDEF
                ) {
                    // signal is a kernel signal
                    call_kernel_signal_handler(signal);
                } else {
                    // signal is a user signal
                    call_user_signal_handler(sig, signal);
                    break;
                }
            }
        }
    }
}

fn call_kernel_signal_handler(signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;
            task_inner.signals ^= SignalFlags::SIGSTOP;
        }
        SignalFlags::SIGCONT => {
            if task_inner.signals.contains(SignalFlags::SIGCONT) {
                task_inner.signals ^= SignalFlags::SIGCONT;
                task_inner.frozen = false;
            }
        }
        _ => {
            task_inner.killed = true;
        }
    }
}

fn call_user_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let handler = task_inner.signal_actions.table[sig].handler;
    if handler != 0 {
        // user handler

        // handle flag
        task_inner.handling_sig = Some(sig);
        task_inner.signals ^= signal;

        // backup trapframe
        let trap_ctx = task_inner.trap_cx();
        task_inner.trap_ctx_backup = Some(*trap_ctx);

        // modify trapframe
        trap_ctx.sepc = handler;

        // put args (a0)
        trap_ctx.x[10] = sig;
    } else {
        // default action
        debug!(
            "[kernel] task::call_user_signal_handler: default action: ignore it or kill process"
        );
    }
}

pub fn check_signals_error_of_current() -> Option<(i32, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    task_inner.signals.check_error()
}
