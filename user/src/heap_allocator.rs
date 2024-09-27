use buddy_system_allocator::LockedHeap;
use core::cell::UnsafeCell;

const USER_HEAP_SIZE: usize = 1024 * 1024 * 8;

struct HeapSpace {
    data: UnsafeCell<[u8; USER_HEAP_SIZE]>,
}

unsafe impl Sync for HeapSpace {}

impl HeapSpace {
    const fn new() -> Self {
        Self {
            data: UnsafeCell::new([0; USER_HEAP_SIZE]),
        }
    }

    fn as_usize(&self) -> usize {
        self.data.get() as *mut u8 as usize
    }
}

static HEAP_SPACE: HeapSpace = HeapSpace::new();

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::<32>::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

pub fn init_heap() {
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP_SPACE.as_usize(), USER_HEAP_SIZE);
    }
}
