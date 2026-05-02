use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
};

use crate::geo::*;
use crate::math::*;
use alloc::boxed::Box;
use alloc::{alloc::alloc, vec::Vec};
use bootloader_api::info::{FrameBuffer as BootFrameBuffer, FrameBufferInfo, PixelFormat};
use core::fmt::Debug;
use spin::Mutex;

use super::font::{Psf2Font, Psf1Font, FONT_DEFAULT, PsfFont};
use super::Color;
use crate::context::GLOBAL_CONTEXT;
use crate::{context::Context, error, info};

pub trait FrameBuffer {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn stride(&self) -> usize;
    fn byte_per_pixel(&self) -> usize;
    fn pixel_format(&self) -> PixelFormat;
    fn buffer(&self) -> &[u8];
    fn buffer_mut(&mut self) -> &mut [u8];

    fn get_pixel(&self, v: Vec2<usize>) -> Color;
    fn set_pixel(&mut self, v: Vec2<usize>, color: Color);

    fn draw_object<T: Arithmetic>(&mut self, object: &impl Object<T>, color: Color) {
        for point in object.points().points {
            self.set_pixel(point.as_usize(), color);
        }
    }

    fn clear_screen(&mut self, color: Color);

    fn move_object<T: Arithmetic, O: Object<T>>(
        &mut self,
        old: &O,
        old_color: Color,
        delta: Vec2<T>,
        remplace_by: Color,
    ) -> O {
        for point in old.points().points {
            self.set_pixel(vec2(point.x.as_usize(), point.y.as_usize()), remplace_by);
        }

        let new_points = old.translate(delta);
        for point in new_points.points().points {
            self.set_pixel(vec2(point.x.as_usize(), point.y.as_usize()), old_color);
        }

        new_points
    }

    fn draw_psf_string(&mut self, pos: Vec2<usize>, string: &str, color: Color, psf_font: &impl PsfFont) {
        let mut x = pos.x;
        let mut y = pos.y;

        for c in string.chars() {
            if c == '\n' {
                x = pos.x;
                y += psf_font.height();
                continue;
            }
            if x >= self.width() {
                x = pos.x;
                y += psf_font.height();
            }

            if y >= self.height() {
                break;
            }
            let glyph = psf_font.get_glyph(c, vec2(x, y)).unwrap();
            self.draw_object(&glyph, color);
            x += psf_font.width();
        }
    }

    #[must_use]
    fn slide_object<T: Arithmetic + core::cmp::Ord>(
        &mut self,
        object: &impl Object<T>,
        dest: Vec2<T>,
        color: Color,
        background: Color,
    ) -> Option<()> {
        let mut rect = object.rect()?;

        while rect.p1 == dest {
            let old = rect;
            self.move_object(
                &old,
                color,
                vec2(T::new(1.0), T::new(1.0)),
                background,
            );

            rect = rect.translate(vec2(T::new(1.0), T::new(1.0)));
        }

        Some(())


    }
}

#[derive(Debug, Clone, Copy)]
pub struct RawFrameBuffer {
    addr: u64,
    info: FrameBufferInfo,
}

unsafe impl Send for RawFrameBuffer {}
unsafe impl Sync for RawFrameBuffer {}

impl RawFrameBuffer {
    pub unsafe fn create_from_raw_addr(addr: u64, info: FrameBufferInfo) -> Self {
        let size = info.width * info.height * info.bytes_per_pixel;
        RawFrameBuffer { addr, info }
    }

    pub unsafe fn alloc(info: FrameBufferInfo) -> Self {
        let size = info.width * info.height * info.bytes_per_pixel;
        let addr = unsafe {
            let ptr = alloc(Layout::from_size_align(size as usize, 4).unwrap());
            if ptr.is_null() {
                panic!("Allocation de framebuffer échouée");
            }
            ptr as u64
        };
        let mut info = info;
        info.byte_len = size as usize;
        info.stride = info.width;
        unsafe { RawFrameBuffer::create_from_raw_addr(addr, info) }
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.info().pixel_format
    }

    pub fn byte_per_pixel(&self) -> usize {
        self.info().bytes_per_pixel
    }

    pub fn stride(&self) -> usize {
        self.info().stride
    }

    pub fn width(&self) -> usize {
        self.info().width
    }

    pub fn height(&self) -> usize {
        self.info().height
    }

    pub fn buffer(&self) -> &[u8] {
        let buffer = self.addr as *const u8;
        unsafe { core::slice::from_raw_parts(buffer, self.info().byte_len) }
    }
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        let buffer = self.addr as *mut u8;
        unsafe { core::slice::from_raw_parts_mut(buffer, self.info().byte_len) }
    }

    pub fn info(&self) -> FrameBufferInfo {
        self.info
    }

    pub unsafe fn free(&mut self) {
        let size = self.info().byte_len;
        unsafe {
            let ptr = self.addr as *mut u8;
            let layout = Layout::from_size_align(size, 4).unwrap();
            alloc::alloc::dealloc(ptr, layout);
        }
        self.addr = 0;
    }
}

impl FrameBuffer for RawFrameBuffer {
    fn width(&self) -> usize {
        self.info().width
    }

    fn height(&self) -> usize {
        self.info().height
    }

    fn stride(&self) -> usize {
        self.info().stride
    }

    fn byte_per_pixel(&self) -> usize {
        self.byte_per_pixel()
    }

    fn pixel_format(&self) -> PixelFormat {
        self.info().pixel_format
    }

    fn buffer(&self) -> &[u8] {
        self.buffer()
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer_mut()
    }

    fn clear_screen(&mut self, color: Color) {
        
        // create a new framebuffer with the same size and pixel format and fill it with the color and copy it to the current framebuffer
        let size = self.info().width * self.info().height * self.info().bytes_per_pixel;
        let mut new_fb = unsafe { RawFrameBuffer::alloc(self.info()) };
        let bpp = new_fb.byte_per_pixel();
        let stride = new_fb.stride();
        let pixel_format = new_fb.pixel_format();
        let buffer = new_fb.buffer_mut();
        let color_value = color.to_u32();
        let mut color_components = [0u8; 4];
        match pixel_format {
            PixelFormat::Bgr => {
                for i in 0..bpp {
                    color_components[i] = ((color_value >> (i * 8)) & 0xFF) as u8;
                }
            }
            PixelFormat::Rgb => {
                for i in 0..bpp {
                    color_components[i] = ((color_value >> (8 * (bpp - 1 - i))) & 0xFF) as u8;
                }
            }
            _ => {}
        }
        for y in 0..self.info().height {
            let row_offset = y * stride * bpp;
            for x in 0..self.info().width {
                let pixel_offset = row_offset + x * bpp;
                for b in 0..bpp {
                    buffer[pixel_offset + b] = color_components[b];
                }
            }
        }
        let self_buffer = self.buffer_mut();
        self_buffer.copy_from_slice(buffer);
        unsafe {
            new_fb.free();
        }
        
    }

    fn get_pixel(&self, v: Vec2<usize>) -> Color {
        if v.x < self.width() && v.y < self.height() {
            let offset = v.y * self.width() + v.x;
            let byte_per_pixel = self.info().bytes_per_pixel;
            let pixel_format = self.pixel_format();
            let buffer = self.buffer();
            let color = &buffer[offset..offset + byte_per_pixel];
            Color::from_pixel_fmt(pixel_format, color)
        } else {
            Color::black()
        }
    }

    fn set_pixel(&mut self, v: Vec2<usize>, color: Color) {
        let [x, y] = v.to_slice();
        let color = color.to_u32();
        let pixel_format = self.pixel_format();
        let bytes_per_pixel = self.byte_per_pixel();
        let stride = self.stride();
        let width = self.width();
        let height = self.height();

        if x >= width || y >= height {
            return;
        }

        let offset = (y * stride + x) as usize * bytes_per_pixel;
        let buffer = self.buffer_mut();
        if offset + bytes_per_pixel > buffer.len() {
            error!(
                "Buffer overflow: offset {}, bytes_per_pixel {}, buffer length {}",
                offset,
                bytes_per_pixel,
                buffer.len()
            );
            return;
        }
        match pixel_format {
            PixelFormat::Bgr => {
                for i in 0..bytes_per_pixel {
                    buffer[offset + i] = ((color >> (i * 8)) & 0xFF) as u8;
                }
            }
            PixelFormat::Rgb => {
                for i in 0..bytes_per_pixel {
                    buffer[offset + i] = ((color >> (8 * (bytes_per_pixel - 1 - i))) & 0xFF) as u8;
                }
            }
            _ => {}
        }
    }
}

pub struct DoubleBuffer {
    pub framebuffer: RawFrameBuffer,
    pub framebuffer2: RawFrameBuffer,
    pub current_framebuffer: usize,
}

impl DoubleBuffer {
    pub fn new(framebuffer: RawFrameBuffer, framebuffer2: RawFrameBuffer) -> Self {
        DoubleBuffer {
            framebuffer,
            framebuffer2,
            current_framebuffer: 0,
        }
    }

    pub fn swap(&mut self) {
        self.current_framebuffer = 1 - self.current_framebuffer;
    }

    pub fn current_mut(&mut self) -> &mut RawFrameBuffer {
        if self.current_framebuffer == 0 {
            &mut self.framebuffer
        } else {
            &mut self.framebuffer2
        }
    }
    pub fn current(&self) -> &RawFrameBuffer {
        if self.current_framebuffer == 0 {
            &self.framebuffer
        } else {
            &self.framebuffer2
        }
    }

    pub fn present(&mut self) {
        if self.current_framebuffer == 0 {
            let back = self.framebuffer.buffer();
            let front = unsafe {
                core::slice::from_raw_parts_mut(
                    self.framebuffer2.addr as *mut u8,
                    self.framebuffer2.info().byte_len,
                )
            };
            front.copy_from_slice(back);
        } else {
            let back = self.framebuffer2.buffer();
            let front = unsafe {
                core::slice::from_raw_parts_mut(
                    self.framebuffer.addr as *mut u8,
                    self.framebuffer.info().byte_len,
                )
            };
            front.copy_from_slice(back);
        }
    }
}

impl FrameBuffer for DoubleBuffer {
    fn clear_screen(&mut self, color: Color) {
        self.framebuffer.clear_screen(color)
    }
    fn width(&self) -> usize {
        self.framebuffer.width()
    }

    fn height(&self) -> usize {
        self.framebuffer.height()
    }

    fn stride(&self) -> usize {
        self.framebuffer.stride()
    }

    fn byte_per_pixel(&self) -> usize {
        self.framebuffer.byte_per_pixel()
    }

    fn pixel_format(&self) -> PixelFormat {
        self.framebuffer.pixel_format()
    }

    fn buffer(&self) -> &[u8] {
        self.current().buffer()
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.current_mut().buffer_mut()
    }

    fn get_pixel(&self, pos: Vec2<usize>) -> Color {
        self.current().get_pixel(pos)
    }

    fn set_pixel(&mut self, pos: Vec2<usize>, color: Color) {
        self.current_mut().set_pixel(pos, color);
    }

}

impl Deref for DoubleBuffer {
    type Target = RawFrameBuffer;

    fn deref(&self) -> &Self::Target {
        self.current()
    }
}

impl DerefMut for DoubleBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.current_mut()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct SwapBuffer {
    front: RawFrameBuffer,
    back: RawFrameBuffer,
    rect_cache: Option<(Rect<usize>, Color)>,
}

impl SwapBuffer {
    pub fn new(front: RawFrameBuffer, back: RawFrameBuffer) -> Self {
        SwapBuffer {
            front,
            back,
            rect_cache: None,
        }
    }

    pub fn clear_screen(&mut self, color: Color) {
        self.front.clear_screen(color);
        self.back.clear_screen(color);
    }

    pub fn swap(&mut self) {
        core::mem::swap(&mut self.front, &mut self.back);
    }

    pub fn slide_rect_animated(
        &mut self,
        rect: Rect<usize>,
        color: Color,
        dx: isize,
        dy: isize,
        background: Color,
        steps: usize,
        delay_ms: u64,
    ) {
        let width = rect.bottom_right().x - rect.top_left().x;
        let height = rect.bottom_right().y - rect.top_left().y;

        let start_top_left_x = rect.top_left().x;
        let start_top_left_y = rect.top_left().y;

        let end_top_left_x = (rect.top_left().x as isize + dx).max(0) as usize;
        let end_top_left_y = (rect.top_left().y as isize + dy).max(0) as usize;

        let step_x = (end_top_left_x as isize - start_top_left_x as isize) as f32 / steps as f32;
        let step_y = (end_top_left_y as isize - start_top_left_y as isize) as f32 / steps as f32;

        for step in 0..=steps {
            let current_top_left_x = start_top_left_x as f32 + step_x * step as f32;
            let current_top_left_y = start_top_left_y as f32 + step_y * step as f32;

            if let Some((old_rect, _)) = self.rect_cache {
                self.clear_rect(&old_rect, background);
            }

            let top_left = vec2(current_top_left_x as usize, current_top_left_y as usize);
            let bottom_right            // Calculer la position actuelle du coin supérieur gauche
 = vec2(top_left.x + width, top_left.y + height);
            let new_rect = Rect {
                p0: top_left,
                p1: bottom_right,
                fill: true
            };

            self.draw_rect(&new_rect, color);

            self.rect_cache = Some((new_rect, color));

            self.blit_to_front();

            if step < steps {
                for _ in 0..10000 * delay_ms as usize {
                    core::hint::spin_loop();
                }
            }
        }
    }

    pub fn slide_rect(
        &mut self,
        rect: &Rect<usize>,
        dx: isize,
        dy: isize,
        color: Color,
        background: Color,
    ) {
        let width = rect.bottom_right().x - rect.top_left().x;
        let height = rect.bottom_right().y - rect.top_left().y;

        let new_top_left_x = Ord::max((rect.top_left().x as isize + dx), 0) as usize;
        let new_top_left_y = Ord::max((rect.top_left().y as isize + dy), 0) as usize;

        let max_x = self.back.width().saturating_sub(width);
        let max_y = self.back.height().saturating_sub(height);
        let new_top_left_x = new_top_left_x.min(max_x);
        let new_top_left_y = new_top_left_y.min(max_y);

        self.clear_rect(rect, background);

        let new_rect = Rect {
            p0: vec2(new_top_left_x, new_top_left_y),
            p1: vec2(new_top_left_x + width, new_top_left_y + height),
            fill: true
        };

        self.draw_rect(&new_rect, color);

        self.blit_to_front();
    }

    pub fn slide_rect_smooth(
        &mut self,
        rect: Rect<usize>,
        color: Color,
        dx: isize,
        dy: isize,
        background: Color,
    ) {
        let distance = libm::sqrt((dx * dx + dy * dy) as f64) as usize;
        let steps = 700;

        let delay_ms = 16; // ~60fps

        self.slide_rect_animated(rect, color, dx, dy, background, steps, delay_ms);
    }

    fn draw_rect(&mut self, rect: &Rect<usize>, color: Color) {
        let bpp = self.back.byte_per_pixel();
        let stride = self.back.stride();
        let back_width = self.back.width();
        let back_height = self.back.height();
        let pixel_format = self.back.pixel_format();
        let buffer = self.back.buffer_mut();

        let width = rect.bottom_right().x - rect.top_left().x;
        let height = rect.bottom_right().y - rect.top_left().y;

        let can_use_fast_fill = bpp == 4 && color.is_uniform();

        let color_value = color.to_u32();
        let mut color_components = [0u8; 4];

        match pixel_format {
            PixelFormat::Bgr => {
                for i in 0..bpp {
                    color_components[i] = ((color_value >> (i * 8)) & 0xFF) as u8;
                }
            }
            PixelFormat::Rgb => {
                for i in 0..bpp {
                    color_components[i] = ((color_value >> (8 * (bpp - 1 - i))) & 0xFF) as u8;
                }
            }
            _ => {}
        }

        for y in 0..height {
            let actual_y = rect.top_left().y + y;
            if actual_y >= back_height {
                break;
            }

            let row_offset = (actual_y * stride + rect.top_left().x) * bpp;

            if can_use_fast_fill {
                let line_start = row_offset;
                let line_end = line_start + width * bpp;

                if line_end <= buffer.len() {
                    for x in 0..width {
                        let pixel_offset = line_start + x * bpp;
                        for b in 0..bpp {
                            buffer[pixel_offset + b] = color_components[b];
                        }
                    }
                }
            } else {
                for x in 0..width {
                    let actual_x = rect.top_left().x + x;
                    if actual_x >= back_width {
                        break;
                    }

                    let offset = row_offset + x * bpp;
                    if offset + bpp <= buffer.len() {
                        for b in 0..bpp {
                            buffer[offset + b] = color_components[b];
                        }
                    }
                }
            }
        }
    }

    fn clear_rect(&mut self, rect: &Rect<usize>, color: Color) {
        self.draw_rect(rect, color);
    }

    fn blit_to_front(&mut self) {
        let back = self.back.buffer();
        let front = unsafe {
            core::slice::from_raw_parts_mut(self.front.addr as *mut u8, self.front.info().byte_len)
        };

        front.copy_from_slice(back);
    }

    fn blit_region_to_front(&mut self, rect: &Rect<usize>) {
        let bpp = self.back.byte_per_pixel();
        let stride = self.back.stride();
        let back = self.back.buffer();
        let front = unsafe {
            core::slice::from_raw_parts_mut(self.front.addr as *mut u8, self.front.info().byte_len)
        };

        let width = rect.bottom_right().x - rect.top_left().x;
        let height = rect.bottom_right().y - rect.top_left().y;

        for y in 0..height {
            let actual_y = rect.top_left().y + y;
            if actual_y >= self.back.height() {
                break;
            }

            let row_offset = (actual_y * stride + rect.top_left().x) * bpp;
            let line_size = width * bpp;

            if row_offset + line_size <= back.len() {
                front[row_offset..row_offset + line_size]
                    .copy_from_slice(&back[row_offset..row_offset + line_size]);
            }
        }
    }

    fn blit_pixels_to_front(&mut self, points: Points<usize>) {
        let bpp = self.back.byte_per_pixel();
        let stride = self.back.stride();
        let back = self.back.buffer();
        let front = unsafe {
            core::slice::from_raw_parts_mut(self.front.addr as *mut u8, self.front.info().byte_len)
        };

        for point in points.points {
            let x = point.x;
            let y = point.y;
            if x >= self.back.width() || y >= self.back.height() {
                continue;
            }

            let row_offset = (y * stride + x) * bpp;
            let pixel_size = bpp;

            if row_offset + pixel_size <= back.len() {
                front[row_offset..row_offset + pixel_size]
                    .copy_from_slice(&back[row_offset..row_offset + pixel_size]);
            }
        }
    }

    fn blit_pixels_to_back(&mut self, points: Points<usize>) {
        let bpp = self.back.byte_per_pixel();
        let stride = self.back.stride();
        let back = self.back.buffer();
        let front = unsafe {
            core::slice::from_raw_parts_mut(self.front.addr as *mut u8, self.front.info().byte_len)
        };

        for point in points.points {
            let x = point.x;
            let y = point.y;
            if x >= self.back.width() || y >= self.back.height() {
                continue;
            }

            let row_offset = (y * stride + x) * bpp;
            let pixel_size = bpp;

            if row_offset + pixel_size <= front.len() {
                front[row_offset..row_offset + pixel_size]
                    .copy_from_slice(&back[row_offset..row_offset + pixel_size]);
            }
        }
    }
}

impl FrameBuffer for SwapBuffer {
    fn width(&self) -> usize {
        self.front.width()
    }
    fn height(&self) -> usize {
        self.front.height()
    }

    fn stride(&self) -> usize {
        self.front.stride()
    }

    fn byte_per_pixel(&self) -> usize {
        self.front.byte_per_pixel()
    }

    fn pixel_format(&self) -> PixelFormat {
        self.front.pixel_format()
    }

    fn buffer(&self) -> &[u8] {
        self.front.buffer()
    }

    fn buffer_mut(&mut self) -> &mut [u8] {
        self.front.buffer_mut()
    }

    fn get_pixel(&self, v: Vec2<usize>) -> Color {
        self.front.get_pixel(v)
    }

    fn set_pixel(&mut self, v: Vec2<usize>, color: Color) {
        self.front.set_pixel(v, color);
    }

    fn clear_screen(&mut self, color: Color) {
        self.front.clear_screen(color);
        self.back.clear_screen(color);
    }

    fn slide_object<T: Arithmetic>(&mut self, object: &impl Object<T>, dest: Vec2<T>, color: Color, background: Color) -> Option<()> {
        self.slide_rect_smooth(object.rect()?.as_usize(), color, dest.x.as_isize(), dest.y.as_isize(), background);
        Some(())
    }
}
