use super::bus::virtio::VirtioHal;
use crate::sync::UPIntrFreeCell;
use alloc::{sync::Arc, vec::Vec};
use core::any::Any;
use embedded_graphics::pixelcolor::Rgb888;
use lazy_static::lazy_static;
use tinybmp::Bmp;
use virtio_drivers::{VirtIOGpu, VirtIOHeader};

lazy_static! {
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWarpper::new());
}

static BMP_DATA: &[u8] = include_bytes!("../../../assets/cursor.bmp");

const VIRTIO7: usize = 0x10007000;

pub trait GpuDevice: Send + Sync + Any {
    #[allow(unused)]
    fn update_cursor(&self);
    fn framebuffer(&self) -> &[u8];
    fn flush(&self);
}

pub struct VirtIOGpuWarpper {
    gpu: UPIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],
}

impl VirtIOGpuWarpper {
    pub fn new() -> Self {
        unsafe {
            let mut virtio =
                VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();

            let frame_buffer = virtio.setup_framebuffer().unwrap();
            let fb = core::slice::from_raw_parts_mut(frame_buffer.as_mut_ptr(), frame_buffer.len());

            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let mut cursor_data = Vec::new();
            for pixel in bmp.as_raw().image_data().chunks(3) {
                let alpha = if pixel == [0xFF, 0xFF, 0xFF] {
                    0x00
                } else {
                    0xFF
                };
                cursor_data.extend(pixel);
                cursor_data.push(alpha)
            }
            virtio
                .setup_cursor(cursor_data.as_slice(), 50, 50, 50, 50)
                .unwrap();

            Self {
                gpu: UPIntrFreeCell::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWarpper {
    fn flush(&self) {
        self.gpu.exclusive_access().flush().unwrap();
    }

    #[allow(clippy::mut_from_ref)]
    fn framebuffer(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.fb.as_ptr(), self.fb.len()) }
    }

    fn update_cursor(&self) {
        unimplemented!()
    }
}
