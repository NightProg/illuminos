use alloc::{string::ToString, sync::Arc};
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr, structures::paging::PageTableFlags};

use crate::{
    allocator::{
        paging::KERNEL_PAGING_MANAGER,
        vma::{MapFlags, ProtFlags, VMA},
    },
    fs::{FileSystem, Inode, InodeKind},
    graphic::framebuffer::RawFrameBuffer,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FbVarScreenInfo {
    pub xres: u32,
    pub yres: u32,
    pub xres_virtual: u32,
    pub yres_virtual: u32,
    pub xoffset: u32,
    pub yoffset: u32,
    pub bits_per_pixel: u32,
    pub grayscale: u32,
    pub red: FbBitfield,
    pub green: FbBitfield,
    pub blue: FbBitfield,
    pub transp: FbBitfield,
    pub nonstd: u32,
    pub activate: u32,
    pub height: u32,
    pub width: u32,
    pub accel_flags: u32,
    pub pixclock: u32,
    pub left_margin: u32,
    pub right_margin: u32,
    pub upper_margin: u32,
    pub lower_margin: u32,
    pub hsync_len: u32,
    pub vsync_len: u32,
    pub sync: u32,
    pub vmode: u32,
    pub rotate: u32,
    pub colorspace: u32,
    pub reserved: [u32; 4],
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FbBitfield {
    pub offset: u32,
    pub length: u32,
    pub msb_right: u32,
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct FbFixScreenInfo {
    pub id: [u8; 16],
    pub smem_start: u64,
    pub smem_len: u32,
    pub type_: u32,
    pub type_aux: u32,
    pub visual: u32,
    pub xpanstep: u16,
    pub ypanstep: u16,
    pub ywrapstep: u16,
    pub line_length: u32,
    pub mmio_start: u64,
    pub mmio_len: u32,
    pub accel: u32,
    pub capabilities: u16,
    pub reserved: [u16; 2],
}

#[repr(u32)]
pub enum IOCTLCommand {
    FBIOGET_VSCREENINFO = 0x4600,
    FBIOGET_FSCREENINFO = 0x4602,
}

impl IOCTLCommand {
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            0x4600 => Some(IOCTLCommand::FBIOGET_VSCREENINFO),
            0x4602 => Some(IOCTLCommand::FBIOGET_FSCREENINFO),
            _ => None,
        }
    }
}

pub struct FramebufferInode {
    fb: RawFrameBuffer,
}

impl FramebufferInode {
    pub fn new(fb: RawFrameBuffer) -> Self {
        Self { fb }
    }
}

impl Inode for FramebufferInode {
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> crate::fs::Result<usize> {
        let fb_addr = self.fb.addr as usize;
        let fb_size = self.fb.pitch * self.fb.height;
        if offset as usize >= fb_size {
            return Ok(0);
        }
        let to_read = core::cmp::min(buf.len(), fb_size - offset as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(
                (fb_addr + offset as usize) as *const u8,
                buf.as_mut_ptr(),
                to_read,
            );
        }
        Ok(to_read)
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> crate::fs::Result<usize> {
        let fb_addr = self.fb.addr as usize;
        let fb_size = self.fb.pitch * self.fb.height;
        if offset as usize >= fb_size {
            return Ok(0);
        }
        let to_write = core::cmp::min(buf.len(), fb_size - offset as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                (fb_addr + offset as usize) as *mut u8,
                to_write,
            );
        }
        Ok(to_write)
    }

    fn size(&mut self) -> u64 {
        (self.fb.pitch as usize * self.fb.height as usize) as u64
    }

    fn kind(&self) -> InodeKind {
        InodeKind::Device
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn mmap(&mut self, vma: &mut VMA) -> crate::fs::Result<()> {
        let size = vma.end - vma.start;
        if size > self.size() {
            return Err("Requested mapping size exceeds framebuffer size".to_string());
        }
        if vma.offset as u64 >= self.size() {
            return Err("Requested mapping offset exceeds framebuffer size".to_string());
        }

        KERNEL_PAGING_MANAGER.lock().as_mut().unwrap().map_memory(
            VirtAddr::new(vma.start),
            size as usize,
            PhysAddr::new(self.fb.phys_addr + vma.offset as u64),
            vma.prot.to_page_table_flags(),
        );

        Ok(())
    }

    fn as_directory(&mut self) -> Option<&dyn crate::fs::DirectoryInode> {
        None
    }

    fn ioctl(&mut self, request: u64, arg: u64) -> crate::fs::Result<u64> {
        let command = IOCTLCommand::from_u64(request).ok_or("Invalid ioctl command")?;
        match command {
            IOCTLCommand::FBIOGET_VSCREENINFO => {
                let var_info = FbVarScreenInfo {
                    xres: self.fb.width as u32,
                    yres: self.fb.height as u32,
                    xres_virtual: self.fb.width as u32,
                    yres_virtual: self.fb.height as u32,
                    bits_per_pixel: (self.fb.pitch * 8 / self.fb.width) as u32,
                    red: FbBitfield {
                        offset: self.fb.pixel_format.get_red_shift() as u32,
                        length: 8,
                        msb_right: 0,
                    },
                    green: FbBitfield {
                        offset: self.fb.pixel_format.get_green_shift() as u32,
                        length: 8,
                        msb_right: 0,
                    },
                    blue: FbBitfield {
                        offset: self.fb.pixel_format.get_blue_shift() as u32,
                        length: 8,
                        msb_right: 0,
                    },
                    transp: FbBitfield {
                        offset: 24,
                        length: 8,
                        msb_right: 0,
                    },
                    ..Default::default()
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        &var_info as *const FbVarScreenInfo as *const u8,
                        arg as *mut u8,
                        size_of::<FbVarScreenInfo>(),
                    );
                }
                Ok(0)
            }
            IOCTLCommand::FBIOGET_FSCREENINFO => {
                let fix_info = FbFixScreenInfo {
                    id: *b"Framebuffer\0\0\0\0\0",
                    smem_start: self.fb.phys_addr,
                    smem_len: (self.fb.pitch * self.fb.height) as u32,
                    visual: 2,
                    line_length: self.fb.pitch as u32,
                    ..Default::default()
                };
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        &fix_info as *const FbFixScreenInfo as *const u8,
                        arg as *mut u8,
                        core::mem::size_of::<FbFixScreenInfo>(),
                    );
                }
                Ok(0)
            }
        }
    }
}
