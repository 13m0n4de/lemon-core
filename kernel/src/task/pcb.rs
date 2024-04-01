use super::{
    id::{pid_alloc, PidHandle, RecycleAllocator},
    manager::{add, insert_into_pid2process},
    tcb::TaskControlBlock,
    SignalFlags,
};
use crate::{
    fs::{find_inode, File, Stdin, Stdout, PROC_INODE},
    mm::{translated_mut_ref, MemorySet, KERNEL_SPACE},
    sync::{Condvar, Mutex, Semaphore, UPIntrFreeCell, UPIntrRefMut},
    trap::{user_handler, Context},
    DEV_NON_BLOCKING_ACCESS,
};
use alloc::{
    format,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};

pub struct ProcessControlBlock {
    pub pid: PidHandle,
    inner: UPIntrFreeCell<ProcessControlBlockInner>,
}

impl ProcessControlBlock {
    pub fn inner_exclusive_access(&self) -> UPIntrRefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }

    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);

        // allocate a pid
        let pid = pid_alloc();
        let process = Arc::new(Self {
            pid,
            inner: unsafe {
                UPIntrFreeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
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
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                })
            },
        });

        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(&process, ustack_base, true));

        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.top();
        drop(task_inner);
        *trap_cx = Context::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            user_handler as usize,
        );

        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(task.clone()));
        drop(process_inner);
        insert_into_pid2process(process.pid(), Arc::clone(&process));

        // add main thread to scheduler
        add(task);
        process
    }

    pub fn pid(&self) -> usize {
        self.pid.0
    }

    #[allow(clippy::similar_names)]
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: &[String]) {
        // only support processes with a single thread
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);

        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();

        // substitute memory_set
        self.inner_exclusive_access().memory_set = memory_set;
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        let task = self.inner_exclusive_access().task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();

        // push arguments on user stack
        let argc = args.len();
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (argc + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=argc)
            .map(|arg| {
                translated_mut_ref(
                    new_token,
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
                *translated_mut_ref(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_mut_ref(new_token, p as *mut u8) = 0;
        }

        // write cmdline
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = false;
        let proc_inode = PROC_INODE
            .find(&self.pid.0.to_string())
            .unwrap_or_else(|| panic!("Failed to find inode for '/proc/{}/'", self.pid.0));
        let cmdline_inode = proc_inode
            .find("cmdline")
            .unwrap_or_else(|| panic!("Failed to find inode for '/proc/{}/cmdline'", self.pid.0));
        cmdline_inode.clear();
        cmdline_inode.write_at(0, args.join(" ").as_bytes());
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;

        // initialize trap_cx
        let mut trap_cx = Context::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.top(),
            user_handler as usize,
        );
        trap_cx.x[10] = argc;
        trap_cx.x[11] = argv_base;
        *task_inner.trap_cx() = trap_cx;
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.inner_exclusive_access();
        // only support processes with a single thread
        assert_eq!(parent_inner.thread_count(), 1);

        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = parent_inner.memory_set.clone();
        // alloc a pid
        let pid = pid_alloc();
        let pid_str = pid.0.to_string();
        // copy fd table
        let new_fd_table = parent_inner.fd_table.clone();

        // create child process PCB
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPIntrFreeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    cwd: parent_inner.cwd.clone(),
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                })
            },
        });
        // add child
        parent_inner.children.push(child.clone());

        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            &child,
            parent_inner
                .task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            false,
        ));

        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(task.clone()));
        drop(child_inner);

        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.trap_cx();
        trap_cx.kernel_sp = task.kstack.top();
        drop(task_inner);

        insert_into_pid2process(child.pid(), child.clone());

        // add this thread to scheduler
        add(task);

        // write proc info
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = false;
        let proc_inode = PROC_INODE
            .create_dir(&pid_str)
            .unwrap_or_else(|| panic!("Failed to create inode for '/proc/{}/'.", pid_str));
        proc_inode.set_default_dirent(PROC_INODE.inode_id());
        let cmdline_inode = proc_inode
            .create("cmdline")
            .unwrap_or_else(|| panic!("Failed to find inode for '/proc/{}/cmdline'.", pid_str));
        if let Some(parent_cmdline_inode) = find_inode(&format!("/proc/{}/cmdline", &self.pid.0)) {
            let mut cmdline = vec![0u8; parent_cmdline_inode.file_size() as usize];
            parent_cmdline_inode.read_at(0, &mut cmdline);
            cmdline_inode.write_at(0, &cmdline);
        }
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;

        child
    }
}

pub struct ProcessControlBlockInner {
    pub is_zombie: bool,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub exit_code: i32,
    pub cwd: String,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub signals: SignalFlags,
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub task_res_allocator: RecycleAllocator,
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
}

impl ProcessControlBlockInner {
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }

    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid);
    }

    pub fn alloc_fd(&mut self) -> usize {
        if let Some(idx) = self.fd_table.iter().position(core::option::Option::is_none) {
            idx
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }

    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = false;
        if let Some(proc_inode) = PROC_INODE.find(&self.pid.0.to_string()) {
            proc_inode.delete("cmdline");
            PROC_INODE.delete(&self.pid.0.to_string());
        }
        *DEV_NON_BLOCKING_ACCESS.exclusive_access() = true;
    }
}
