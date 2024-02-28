mod address;
mod heap_allocator;
mod page_table;

/// Initiate heap allocator
pub fn init() {
    heap_allocator::init_heap();
}
