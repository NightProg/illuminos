use alloc::vec;
use alloc::vec::Vec;
use bootloader_api::info::FrameBufferInfo;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{context::GLOBAL_CONTEXT, info};

use super::framebuffer::{FrameBuffer, VirtualFrameBuffer};

lazy_static! {
    pub static ref WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new());
}

pub struct Widget {
    virt: VirtualFrameBuffer,
    widget_id: usize,
}

impl Widget {
    pub fn new(virt: VirtualFrameBuffer, widget_id: usize) -> Self {
        let mut virt = virt;
        let width = virt.frame_buffer().width();
        let height = virt.frame_buffer().height();
        virt.frame_buffer_mut().draw_rect_no_fill(0, 0, width, height, 0x808080);
        Widget { virt, widget_id }
    }

    pub fn get_widget_id(&self) -> usize {
        self.widget_id
    }

    pub fn get_virt(&self) -> &VirtualFrameBuffer {
        &self.virt
    }

    pub fn get_virt_mut(&mut self) -> &mut VirtualFrameBuffer {
        &mut self.virt
    }

    pub fn move_to(&mut self, x: usize, y: usize) {
        let (old_x, old_y) = self.virt.get_position();
        let frame_buffer = self.virt.frame_buffer_mut();
        let width = frame_buffer.width();
        let height = frame_buffer.height();

        let mut v = Vec::new();

        for i in 0..width {
            for j in 0..height {
                let pixel = frame_buffer.get_pixel(old_x + i, old_y + j);
                v.push(pixel);
                frame_buffer.draw_pixel(old_x + i, old_y + j, 0x000000);
            }
        } 

        for i in 0..width {
            for j in 0..height {
                let pixel = v[j + i * width];
                frame_buffer.draw_pixel(x + i, y + j, pixel);
            }
        }
        let frame_buffer = self.virt.frame_buffer_mut();
        let width = frame_buffer.width();
        let height = frame_buffer.height();
        frame_buffer.draw_rect_no_fill(x, y, width - x, height - y, 0x808080);

        self.virt.set_position(x, y);

        self.sync();

    }

    pub fn sync(&mut self) {
        self.virt.render(GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap());
    }
}

pub struct Window {
    virt: VirtualFrameBuffer,
    window_id: usize,
}

impl Window {
    pub fn new(virt: VirtualFrameBuffer, x: usize, y: usize, window_id: usize) -> Self {
        let mut virt = virt;
        let width = virt.frame_buffer().width();
        let height = virt.frame_buffer().height();

        info!("Creating window at ({}, {}) with size ({}, {})", x, y, width, height);
        virt.frame_buffer_mut().draw_rect_no_fill(x, y, width, height, 0xFFFFFF);
        info!("Window created");
        Window { virt, window_id }
    }

    pub fn clear(&mut self) {
        let frame_buffer = self.virt.frame_buffer_mut();
        let width = frame_buffer.width();
        let height = frame_buffer.height();
        for i in 0..width {
            for j in 0..height {
                frame_buffer.draw_pixel(i, j, 0x000000);
            }
        }
    }

    pub fn get_window_id(&self) -> usize {
        self.window_id
    }

    pub fn get_virt(&self) -> &VirtualFrameBuffer {
        &self.virt
    }

    pub fn get_virt_mut(&mut self) -> &mut VirtualFrameBuffer {
        &mut self.virt
    }

    pub fn move_to(&mut self, x: usize, y: usize) {
        let (old_x, old_y) = self.virt.get_position();
        let frame_buffer = self.virt.frame_buffer_mut();
        let width = frame_buffer.width();
        let height = frame_buffer.height();

        // let mut v: Vec<_> = Vec::new();

        info!("Moving window from ({}, {}) to ({}, {})", old_x, old_y, x, y);
        let mut global_context = GLOBAL_CONTEXT.lock();
        let global_framebuffer = global_context.framebuffer.as_mut().unwrap();
        for i in 0..width+1 {
            for j in 0..height {
                global_framebuffer.draw_pixel(old_x + i, old_y + j, 0x000000);
            }
        } 
        self.virt.set_position(x, y);

    }

    pub fn sync(&mut self) {
        self.virt.render(GLOBAL_CONTEXT.lock().framebuffer.as_mut().unwrap());
    }


}

pub struct WindowManager {
    windows: Vec<Window>,
    current_window: usize,
}

impl WindowManager {
    pub fn head() -> Window {
        let f = VirtualFrameBuffer::global();
        Window::new(f, 0, 0, 0)
    }
    pub fn new() -> Self {
        WindowManager {
            windows: vec![],
            current_window: 0,
        }
    }

    pub fn init(&mut self) {
        let window = WindowManager::head();
        self.windows.push(window);
        self.current_window = 1;
    }

    pub fn new_window(&mut self, width: usize, height: usize, x: usize, y: usize) -> usize {
        let global_context = GLOBAL_CONTEXT.lock();
        let global_framebuffer = global_context.framebuffer.as_ref().unwrap();
        let framebuffer = unsafe {
            FrameBuffer::alloc(
                FrameBufferInfo {
                    width,
                    height,
                    ..global_framebuffer.info()
                }
            )
        };
        let mut virt_framebuffer = VirtualFrameBuffer::new(
            framebuffer
        );

        virt_framebuffer.set_position(x, y);
        let window = Window::new(virt_framebuffer, self.windows.len(), x, y);
        self.windows.push(window);
        self.current_window = self.windows.len() - 1;
        self.windows[self.current_window].sync();
        self.current_window
    }

    pub fn add_window(&mut self, window: Window) {
        self.windows.push(window);
    }

    pub fn get_window(&self, id: usize) -> &Window {
        if id >= self.windows.len() {
            panic!("Window id out of bounds: {}", id);
        }
        &self.windows[id]
    }

    pub fn get_window_mut(&mut self, id: usize) -> &mut Window {
        if id >= self.windows.len() {
            panic!("Window id out of bounds: {}", id);
        }
        &mut self.windows[id]
    }

    pub fn get_current_window(&self) -> &Window {
        &self.windows[self.current_window]
    }

    pub fn get_current_window_mut(&mut self) -> &mut Window {
        &mut self.windows[self.current_window]
    }

    pub fn sync(&mut self) {
        for window in &mut self.windows {
            window.sync();
        }
    }

}