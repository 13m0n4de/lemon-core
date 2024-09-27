#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

extern crate alloc;

use core::cell::UnsafeCell;

use user_lib::{
    process::exit,
    sync::{
        condvar_create, condvar_signal, condvar_wait, mutex_blocking_create, mutex_lock,
        mutex_unlock, sleep,
    },
    thread::{thread_create, waittid},
};

struct ConditionFlag {
    value: UnsafeCell<usize>,
}

impl ConditionFlag {
    const fn new(value: usize) -> Self {
        ConditionFlag {
            value: UnsafeCell::new(value),
        }
    }

    unsafe fn get(&self) -> usize {
        *self.value.get()
    }

    unsafe fn set(&self, new_value: usize) {
        *self.value.get() = new_value;
    }
}

unsafe impl Sync for ConditionFlag {}

static A: ConditionFlag = ConditionFlag::new(0);

const CONDVAR_ID: usize = 0;
const MUTEX_ID: usize = 0;

unsafe fn first() -> ! {
    sleep(10);
    println!("First work, Change A --> 1 and wakeup Second");
    mutex_lock(MUTEX_ID);
    A.set(1);
    condvar_signal(CONDVAR_ID);
    mutex_unlock(MUTEX_ID);
    exit(0)
}

unsafe fn second() -> ! {
    println!("Second want to continue,but need to wait A=1");
    mutex_lock(MUTEX_ID);
    while A.get() == 0 {
        println!("Second: A is {}", A.get());
        condvar_wait(CONDVAR_ID, MUTEX_ID);
    }
    println!("A is {}, Second can work now", A.get());
    mutex_unlock(MUTEX_ID);
    exit(0)
}

#[no_mangle]
pub extern "Rust" fn main() -> i32 {
    // create condvar & mutex
    assert_eq!(condvar_create() as usize, CONDVAR_ID);
    assert_eq!(mutex_blocking_create() as usize, MUTEX_ID);
    // create threads
    let threads = &[
        thread_create(first as usize, 0),
        thread_create(second as usize, 0),
    ];
    // wait for all threads to complete
    for thread in threads {
        waittid(*thread as usize);
    }
    println!("test_condvar passed!");
    0
}
