// use core::ffi::usize;

// use bootloader_api::info::FrameBufferInfo;

// use crate::{
//     context::GLOBAL_CONTEXT,
//     graphic::{self, framebuffer::FrameBuffer},
// };

// #[repr(C)]
// pub struct CFrameBuffer {
//     framebuffer: FrameBuffer,
// }

// impl CFrameBuffer {
//     pub fn new(fb: FrameBuffer) -> Self {
//         Self { framebuffer: fb }
//     }

//     pub unsafe fn framebuffer(&self) -> &FrameBuffer {
//         &self.framebuffer
//     }

//     pub unsafe fn framebuffer_mut(&mut self) -> &mut FrameBuffer {
//         &mut self.framebuffer
//     }
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_global() -> *mut CFrameBuffer {
//     let context = GLOBAL_CONTEXT.lock();

//     let fb = context.framebuffer.as_mut().unwrap();

//     let fb = CFrameBuffer::new(fb);
//     Box::into_raw(Box::new(fb))
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_new(
//     addr: usize,
//     info: FrameBufferInfo,
// ) -> *mut CFrameBuffer {
//     let mut fb = FrameBuffer::create_from_raw_addr(addr as usize, info);
//     Box::into_raw(Box::new(CFrameBuffer::new(fb)))
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_free(fb: *mut CFrameBuffer) {
//     let fb = Box::from_raw(fb);
//     drop(fb);
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_draw_pixel(
//     fb: *mut CFrameBuffer,
//     x: usize,
//     y: usize,
//     color: u32,
// ) {
//     let fb = &mut *fb;
//     fb.framebuffer_mut().draw_pixel(x, y, color);
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_clear(fb: *mut CFrameBuffer, color: u32) {
//     let fb = &mut *fb;
//     fb.framebuffer_mut().clear_screen(color);
// }

// #[unsafe(no_mangle)]
// unsafe extern "C" fn illuminos_framebuffer_draw_rect(
//     fb: *mut CFrameBuffer,
//     x: usize,
//     y: usize,
//     width: usize,
//     height: usize,
//     color: u32,
// ) {
//     let fb = &mut *fb;
//     fb.framebuffer_mut().draw_rect(x, y, width, height, color);
// }

// static FRAMEBUFFER_ADDR: u64 = graphic::vram::VRAM_VIRT_ADDR;

// #[repr(C)]
// pub struct GraphicHandle {
//     illuminos_framebuffer_global: unsafe extern "C" fn() -> *mut CFrameBuffer,
//     illuminos_framebuffer_new:
//         unsafe extern "C" fn(addr: usize, info: FrameBufferInfo) -> *mut CFrameBuffer,
//     illuminos_framebuffer_free: unsafe extern "C" fn(fb: *mut CFrameBuffer),
//     illuminos_framebuffer_draw_pixel:
//         unsafe extern "C" fn(fb: *mut CFrameBuffer, x: usize, y: usize, color: u32),
//     illuminos_framebuffer_clear: unsafe extern "C" fn(fb: *mut CFrameBuffer, color: u32),
//     illuminos_framebuffer_draw_rect: unsafe extern "C" fn(
//         fb: *mut CFrameBuffer,
//         x: usize,
//         y: usize,
//         width: usize,
//         height: usize,
//         color: u32,
//     ),
// }

// impl GraphicHandle {
//     pub fn new() -> Self {
//         Self {
//             illuminos_framebuffer_global,
//             illuminos_framebuffer_new,
//             illuminos_framebuffer_free,
//             illuminos_framebuffer_draw_pixel,
//             illuminos_framebuffer_clear,
//             illuminos_framebuffer_draw_rect,
//         }
//     }
// }
