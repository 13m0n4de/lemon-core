mod address;
mod heap_allocator;

/// Initiate heap allocator
pub fn init() {
    heap_allocator::init_heap();
}
