//! Block drivers

mod virtio_blk;

use alloc::sync::Arc;
use easy_fs::BlockDevice;
use lazy_static::lazy_static;

use crate::board::BlockDeviceImpl;

#[allow(clippy::module_name_repetitions)]
pub use virtio_blk::VirtIOBlock;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{test, test_assert};

    test!(test_block_device, {
        let block_device = BLOCK_DEVICE.clone();
        let mut write_buffer = [0u8; 512];
        let mut read_buffer = [0u8; 512];
        for i in 0..512 {
            write_buffer.fill(i as u8);
            block_device.write_block(i, &write_buffer);
            block_device.read_block(i, &mut read_buffer);
            test_assert!(write_buffer == read_buffer);
        }
        Ok("passed")
    });
}
