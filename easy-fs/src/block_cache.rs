use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    block_dev::BlockDevice,
    config::{BLOCK_CACHE_SIZE, BLOCK_SIZE},
};

/// Cached block inside memory
pub struct BlockCache {
    /// cached block data
    cache: [u8; BLOCK_SIZE],
    /// underlying block id
    block_id: usize,
    /// underlying block device
    block_device: Arc<dyn BlockDevice>,
    /// whether the block is dirty
    modified: bool,
}

impl BlockCache {
    /// Load a new [`BlockCache`] from disk
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id, &mut cache);
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }

    /// Get the address of an offset inside the cached block data
    #[inline]
    fn addr_of_offset(&self, offset: usize) -> usize {
        core::ptr::from_ref(&self.cache[offset]) as usize
    }

    pub fn as_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe { &*(addr as *const T) }
    }

    pub fn as_mut_ref<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        self.modified = true;
        let addr = self.addr_of_offset(offset);
        unsafe { &mut *(addr as *mut T) }
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache);
        }
    }

    #[inline]
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.as_ref(offset))
    }

    #[inline]
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.as_mut_ref(offset))
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

pub struct BlockCacheManager {
    queue: Vec<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    #[inline]
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn get(
        &mut self,
        block_id: usize,
        block_device: &Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some((_, cache)) = self.queue.iter().find(|(id, _)| *id == block_id) {
            cache.clone()
        } else {
            // substitute
            if self.queue.len() == BLOCK_CACHE_SIZE {
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, (_, cache))| Arc::strong_count(cache) == 1)
                {
                    self.queue.swap_remove(idx);
                } else {
                    panic!("Run out of BlockCache");
                }
            }
            // load block into mem and push back
            let block_cache = Arc::new(Mutex::new(BlockCache::new(block_id, block_device.clone())));
            self.queue.push((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    /// The global block cache manager
    static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(BlockCacheManager::new());
}

#[inline]
pub fn get(block_id: usize, block_device: &Arc<dyn BlockDevice>) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get(block_id, block_device)
}

#[inline]
pub fn sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in &manager.queue {
        cache.lock().sync();
    }
}
