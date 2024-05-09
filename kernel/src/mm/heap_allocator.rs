//! The global allocator

use crate::config::KERNEL_HEAP_SIZE;
use buddy_system_allocator::LockedHeap;

/// Heap allocator instance
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

/// Heap space (`[u8; KERNEL_HEAP_SIZE]`)
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];

/// Panic when headp allocation error occurs
#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Headp allocation error, layout = {:?}", layout);
}

/// Initialize heap allocator
pub fn init() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
}

#[cfg(test)]
mod test {
    use crate::{test, test_assert};

    test!(test_heap_allocator, {
        use alloc::boxed::Box;
        use alloc::vec::Vec;

        extern "C" {
            fn sbss();
            fn ebss();
        }

        let bss_range = sbss as usize..ebss as usize;
        let a = Box::new(5);
        test_assert!(*a == 5);
        test_assert!(bss_range.contains(&(core::ptr::from_ref(a.as_ref()) as usize)));
        drop(a);

        let mut v: Vec<usize> = Vec::new();
        for i in 0..500 {
            v.push(i);
        }
        for (i, val) in v.iter().take(500).enumerate() {
            test_assert!(*val == i);
        }
        test_assert!(bss_range.contains(&(v.as_ptr() as usize)));
        drop(v);

        Ok("passed")
    });
}
