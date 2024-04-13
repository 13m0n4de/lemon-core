use crate::syscall::{sys_framebuffer, sys_framebuffer_flush};
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point},
    pixelcolor::{Rgb888, RgbColor},
    prelude::Size,
    Pixel,
};

pub const VIRTGPU_XRES: u32 = 1280;
pub const VIRTGPU_YRES: u32 = 800;
pub const VIRTGPU_LEN: usize = (VIRTGPU_XRES * VIRTGPU_YRES * 4) as usize;

pub fn framebuffer() -> isize {
    sys_framebuffer()
}

pub fn framebuffer_flush() -> isize {
    sys_framebuffer_flush()
}

pub struct Display {
    pub size: Size,
    pub fb: &'static mut [u8],
}

impl Display {
    pub fn new(size: Size) -> Self {
        let fb_ptr = framebuffer() as *mut u8;
        let fb = unsafe { core::slice::from_raw_parts_mut(fb_ptr, VIRTGPU_LEN) };
        Self { size, fb }
    }

    pub fn framebuffer(&mut self) -> &mut [u8] {
        self.fb
    }

    pub fn paint_on_framebuffer(&mut self, p: impl FnOnce(&mut [u8])) {
        p(self.framebuffer());
        framebuffer_flush();
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        self.size
    }
}

impl DrawTarget for Display {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        pixels.into_iter().for_each(|Pixel(Point { x, y }, color)| {
            let idx = (y * VIRTGPU_XRES as i32 + x) as usize * 4;
            if idx + 2 < self.fb.len() {
                self.fb[idx] = color.b();
                self.fb[idx + 1] = color.g();
                self.fb[idx + 2] = color.r();
            }
        });
        framebuffer_flush();
        Ok(())
    }
}
