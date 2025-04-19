use goblin::elf::Elf;
use crate::allocator::paging::PagingManager;
use x86_64::{VirtAddr, structures::paging::{PageSize, Size4KiB, PageTableFlags, Page, page_table::PageTableEntry}};
use core::alloc::Layout;
use alloc::vec::Vec;
use crate::libc::OsHandle;
use crate::info;
use crate::println_serial;
use alloc::boxed::Box;
use crate::thread::Process;

#[derive(Debug)]
pub enum ProgLoaderError {
    GoblinError(goblin::error::Error),
    IsNotExe
}

#[derive(Debug)]
pub struct ProgLoader<'a> {
    elf: Elf<'a>,
    buffer: &'a [u8]
}



impl<'a> ProgLoader<'a> {
    pub fn from_bytes(buffer: &'a [u8]) -> Result<Self, ProgLoaderError> {
        let elf = Elf::parse(buffer.as_ref()).map_err(ProgLoaderError::GoblinError)?;

        Ok(Self {
            elf,
            buffer
        })
    }

    pub fn map_memory(&mut self, paging_manager: &mut PagingManager) -> Option<()> {
        for pheader in self.elf.program_headers.iter() {
            if pheader.p_type == goblin::elf64::program_header::PT_LOAD {
                let align = Size4KiB::SIZE;
                let page_count = ((pheader.p_memsz + align  - 1) & !(align - 1)) / align;
                let mut pages: Vec<Page> = Vec::new();

                for i in 0..page_count {
                    let frame = paging_manager.allocate_frame()?;
                    let frame_addr = frame.start_address();

                    let virt_addr = VirtAddr::new(pheader.p_vaddr + i * align);

                    let flags = {
                        let mut flags = PageTableFlags::WRITABLE | PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;


                        if pheader.is_executable() {

                        }

                        flags
                    };

                    paging_manager.map_memory(
                        virt_addr, pheader.p_memsz as usize, frame_addr, flags
                    );
                    pages.push(Page::containing_address(virt_addr));
                }


                let poffset = pheader.p_offset as usize;
                let pfilesz = pheader.p_filesz as usize;
                let data = &self.buffer[poffset..poffset + pfilesz];
                let raw_ptr = unsafe {
                    pheader.p_vaddr as *mut u8
                };


                unsafe {
                    raw_ptr.copy_from(data.as_ptr(), data.len());
                }
            }
        }

        return Some(());
    }

    pub fn execute(&mut self, paging_manager: &mut PagingManager) -> Result<(), ProgLoaderError> {
        if self.elf.header.e_type != goblin::elf64::header::ET_EXEC {
            return Err(ProgLoaderError::IsNotExe)
        }

        self.map_memory(paging_manager);

        println_serial!("{:?}", paging_manager.mapper.phys_offset());

        let mut res = Process::create_user_page_table(paging_manager).unwrap();
        let mut process =  Process::spawn_user(
            res,
            1024 * 1024,
            1024 * 1024 * 10,
            VirtAddr::new(self.elf.header.e_entry),
            paging_manager
        ).unwrap();

        unsafe {
            process.execute();
        };

/*

        let memory = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(4096 * 10, 8).unwrap())
        };

        let addr = memory.addr();
        type EntryFn = extern "C" fn(OsHandle) -> !;


        use core::arch::asm;

        let entry: EntryFn = unsafe {
            core::mem::transmute(self.elf.header.e_entry)
        };
        let mut os_handle = OsHandle::new();

        let old_stack: u64;

        unsafe {
            asm!("mov {}, rsp", out(reg) old_stack, options(nostack, nomem));
            asm!("mov rsp, {}", in(reg) addr, options(nostack, nomem));
        };

        entry(os_handle);

        unsafe {
            asm!("mov rsp, {}", in(reg) old_stack, options(nostack, nomem));
         }
        */
        info!("FINISH");

        Ok(())
    }
}
