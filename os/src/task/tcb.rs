use super::{
    context::Context as TaskContext,
    pid::{alloc as pid_alloc, KernelStack, PidHandle},
    SignalActions, SignalFlags, Status,
};
use crate::{
    config::TRAP_CONTEXT,
    fs::{find_inode, File, Stdin, Stdout},
    mm::{translated_mut_ref, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE},
    sync::UPSafeCell,
    trap::{user_handler, Context as TrapContext},
};
use alloc::{
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use core::cell::RefMut;

pub struct TaskControlBlock {
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = Status::Ready;

        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.top();

        // push a task context with goes to trap_return to the top of kernel stack
        let task_control_block = Self {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: user_sp,
                    task_status,
                    task_cx: TaskContext::leave_trap(kernel_stack_top),
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    cwd: String::from("/"),
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr (stdout)
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    signal_mask: SignalFlags::empty(),
                    signal_actions: SignalActions::default(),
                    killed: false,
                    frozen: false,
                    handling_sig: None,
                    trap_ctx_backup: None,
                })
            },
        };

        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            user_handler as usize,
        );

        task_control_block
    }

    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }

    #[allow(clippy::similar_names)]
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        // ---- access parent PCB exclusively
        let mut parent_inner = self.inner_exclusive_access();

        // copy user space (include trap context)
        let memory_set = parent_inner.memory_set.clone();
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // alloc a pid and a kernel stack in kernel space
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.top();

        // write proc info
        let procs_inode = find_inode("/proc").expect("Failed to find inode for '/proc/'.");
        let proc_inode = procs_inode
            .create_dir(&pid_handle.0.to_string())
            .unwrap_or_else(|| panic!("Failed to create inode for '/proc/{}/'.", pid_handle.0));
        proc_inode.set_default_dirent(procs_inode.inode_id());
        let cmdline_inode = proc_inode.create("cmdline").unwrap_or_else(|| {
            panic!("Failed to find inode for '/proc/{}/cmdline'.", pid_handle.0)
        });

        if let Some(parent_cmdline_inode) = find_inode(&format!("/proc/{}/cmdline", &self.pid.0)) {
            let mut cmdline = vec![0u8; parent_cmdline_inode.file_size() as usize];
            parent_cmdline_inode.read_at(0, &mut cmdline);
            cmdline_inode.write_at(0, &cmdline);
        }

        let new_fd_table = parent_inner.fd_table.clone();

        let task_control_block = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    trap_cx_ppn,
                    base_size: parent_inner.base_size,
                    task_cx: TaskContext::leave_trap(kernel_stack_top),
                    task_status: Status::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    cwd: parent_inner.cwd.clone(),
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    signal_mask: parent_inner.signal_mask,
                    signal_actions: parent_inner.signal_actions,
                    killed: false,
                    frozen: false,
                    handling_sig: None,
                    trap_ctx_backup: None,
                })
            },
        });

        // add child
        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx

        // **** access children PCB exclusively
        let trap_cx = task_control_block.inner_exclusive_access().trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        // return
        task_control_block
        // ---- release parent PCB automatically
        // **** release children PCB automatically
    }

    #[allow(clippy::similar_names)]
    pub fn exec(&self, elf_data: &[u8], args: &[String]) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, mut user_sp, entry_point) = MemorySet::from_elf(elf_data);

        let cmdline_inode = find_inode(&format!("/proc/{}/cmdline", self.pid.0))
            .unwrap_or_else(|| panic!("Failed to find inode for '/proc/{}/cmdline'", self.pid.0));
        cmdline_inode.clear();
        cmdline_inode.write_at(0, args.join(" ").as_bytes());

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        // push arguments on user stack
        let argc = args.len();
        user_sp -= (argc + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=argc)
            .map(|arg| {
                translated_mut_ref(
                    memory_set.token(),
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[argc] = 0;
        for i in 0..argc {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_mut_ref(memory_set.token(), p as *mut u8) = *c;
                p += 1;
            }
            *translated_mut_ref(memory_set.token(), p as *mut u8) = 0;
        }

        // **** access inner exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = memory_set;
        // update trap_cx ppn
        inner.trap_cx_ppn = trap_cx_ppn;
        // initialize base_size
        inner.base_size = user_sp;
        // initialize trap_cx
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.top(),
            user_handler as usize,
        );

        trap_cx.x[10] = argc;
        trap_cx.x[11] = argv_base;
        *inner.trap_cx() = trap_cx;
        // **** release inner automatically
    }

    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,

    pub base_size: usize,

    pub task_status: Status,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,

    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,

    pub cwd: String,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,

    pub signals: SignalFlags,
    pub signal_mask: SignalFlags,
    pub signal_actions: SignalActions,
    pub killed: bool,
    pub frozen: bool,
    pub handling_sig: Option<usize>,
    pub trap_ctx_backup: Option<TrapContext>,
}

impl TaskControlBlockInner {
    pub fn trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.as_mut_ref()
    }

    pub fn user_token(&self) -> usize {
        self.memory_set.token()
    }

    fn status(&self) -> Status {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.status() == Status::Zombie
    }

    #[allow(unused)]
    pub fn is_ready(&self) -> bool {
        self.status() == Status::Ready
    }

    #[allow(unused)]
    pub fn is_running(&self) -> bool {
        self.status() == Status::Running
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(idx) = self.fd_table.iter().position(core::option::Option::is_none) {
            idx
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

impl Drop for TaskControlBlock {
    #[allow(clippy::similar_names)]
    fn drop(&mut self) {
        let procs_inode = find_inode("/proc").expect("Failed to find inode for '/proc/'.");
        if let Some(proc_inode) = procs_inode.find(&self.pid.0.to_string()) {
            proc_inode.delete("cmdline");
            procs_inode.delete(&self.pid.0.to_string());
        }
    }
}
