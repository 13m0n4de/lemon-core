use crate::config::*;
use crate::trap::TrapContext;
use core::arch::asm;

// stack for kernel mode
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

// stack for user mode
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

// global instance of `KernelStack`
// will be placed on the `.rodata`, but currently the `.rodata` segment is RWX
static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

// global instance of `UserStack`
// will be placed on the `.rodata`, but currently the `.rodata` segment is RWX
static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    pub fn push_context(&self, cx: TrapContext) -> usize {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        cx_ptr as usize
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

pub fn load_apps() {
    extern "C" {
        fn _num_app();
    }

    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = unsafe { num_app_ptr.read_volatile() };
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };

    for app_id in 0..num_app {
        let app_base = APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT;
        // clear region
        unsafe {
            core::slice::from_raw_parts_mut(app_base as *mut u8, APP_SIZE_LIMIT).fill(0);
        }
        // load app from data section to memory
        let app_len = app_start[app_id + 1] - app_start[app_id];
        let app_src =
            unsafe { core::slice::from_raw_parts(app_start[app_id] as *const u8, app_len) };
        let app_dst = unsafe { core::slice::from_raw_parts_mut(app_base as *mut u8, app_len) };
        app_dst.copy_from_slice(app_src);
        // Memory fence about fetching the instruction memory
        // It is guaranteed that a subsequent instruction fetch must
        // observes all previous writes to the instruction memory.
        // Therefore, fence.i must be executed after we have loaded
        // the code of the next app into the instruction memory.
        // See also: riscv non-priv spec chapter 3, 'Zifencei' extension.
        unsafe { asm!("fence.i") };
    }
}

pub fn init_app_cx(app_id: usize) -> usize {
    KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
        APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT,
        USER_STACK[app_id].get_sp(),
    ))
}
