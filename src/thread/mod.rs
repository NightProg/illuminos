use core::arch::asm;
use x86_64::{VirtAddr, PhysAddr};
use core::alloc::Layout;
use alloc::vec::Vec;
use crate::allocator::{memory::{HEAP_SIZE, HEAP_START, reserve_memory}, paging::PagingManager};
use core::arch::global_asm;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::math;
use x86_64::structures::paging::{Size4KiB, PageSize, PhysFrame, PageTableFlags, OffsetPageTable, Page, PageTable, Mapper, Translate};
use x86_64::registers::control::{Cr3, Cr3Flags};
use crate::error;
use crate::println_serial;

use crate::gdt::GDT;

pub static PID: AtomicUsize = AtomicUsize::new(1);


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pid(usize);

impl Pid {
    pub fn new() -> Self {
        let pid = PID.load(Ordering::SeqCst);
        PID.fetch_add(1, Ordering::SeqCst);
        Pid(pid)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Stack {
    stack_base: u64, // rbp
    stack_top: u64 // rsp
}


impl Stack {
    pub fn allocate(size: usize) -> Stack {
        let ptr = unsafe {
            alloc::alloc::alloc(
                Layout::from_size_align(
                    size,
                    16
                ).unwrap()
            )
        };


        let stack_base = ptr.addr() as u64;
        let stack_top = stack_base + size as u64;

        Stack {
            stack_base,
            stack_top
        }
    }

    pub fn allocate_with(pm: &mut PagingManager, addr: usize, page_count: usize) -> Option<Stack> {
        for i in 0..page_count {
            let page_addr = addr + i * 0x1000;
            let frame = pm.allocate_frame()?;
            pm.map_page(
                Page::containing_address(VirtAddr::new(page_addr as u64)),
                frame,
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            );
        }

        Some(Stack {
            stack_base: addr as u64,
            stack_top: (addr + page_count * 0x1000) as u64,
        })
    }

    pub fn get() -> Stack {
        let rsp: u64;
        let rbp: u64;
        unsafe {
            asm!("mov {}, rsp", out(reg) rsp, options(nostack, nomem));
            asm!("mov {}, rbp", out(reg) rbp, options(nostack, nomem));
        }

        Stack {
            stack_base: rbp,
            stack_top: rsp
        }
    }


}


#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct Registers {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64
}

pub enum ThreadState {
    Ready,
    Running,
    Terminated
}

pub struct Thread {
    stack: Stack,
    state: ThreadState,
    priority: usize,
    regs: Registers,
    pid: Pid,
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ring {
    Ring0,
    Ring3
}

pub struct ProcessMemoryContext<'a> {
    pub paging_manager: PagingManager<'a>,
    page_table_addr: PhysFrame,
    entry_point: VirtAddr,
    stack: Stack
}



pub struct Process<'a> {
    threads: Vec<Thread>,
    pub memory: ProcessMemoryContext<'a>,
    ring: Ring,
}

impl<'a> Process<'a> {
    pub fn kernel(paging_manager: PagingManager<'a>) -> Process<'a> {
        let mut threads = Vec::new();

        let stack = Stack::get();
        let process_memory_context = ProcessMemoryContext {
            paging_manager,
            entry_point: VirtAddr::new(0x0),
            page_table_addr: PhysFrame::from_start_address(PhysAddr::new(0x0)).unwrap(),
            stack
        };
        Process { threads, memory: process_memory_context, ring: Ring::Ring0 }
    }

    pub fn create_user_page_table(pm: &mut PagingManager) -> Option<(&'a mut PageTable, PhysFrame)> {
/*        let alloc = unsafe {
            alloc::alloc::alloc(
                Layout::from_size_align(core::mem::size_of::<PageTable>(), 4096).ok()?
            )
        };

        let page: Page<Size4KiB> = Page::containing_address(
            VirtAddr::new(alloc.addr() as u64)
        );
        let frame = pm.mapper.translate_page(page).ok()?;
        let pml4 = unsafe {
            let ptr = alloc as *mut PageTable;
            ptr.write(PageTable::new());
            &mut *ptr
        };

        let x = unsafe {
            alloc as *mut PageTable
        };

        println_serial!("{:?}", pm.mapper.phys_offset());
         */

        let frame = pm.allocate_frame()?;
        let page = Page::containing_address(VirtAddr::new(0x3333_3333_3333));
        pm.map_page(page, frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);

        let start_addr = page.start_address().as_u64();
        let pml4 = unsafe {
            let ptr = start_addr as *mut PageTable;
            ptr.write(PageTable::new());
            &mut *ptr
        };



        for i in 0..512 {
            pml4[i] = pm.mapper.level_4_table()[i].clone();
        }
        return Some((pml4, frame));
        //return Some((pml4.clone(), frame));


    }

    pub fn spawn_user(
        user_page_table: (&'a mut PageTable, PhysFrame),
        stack_size: usize,
        mem_size: usize,
        entry_point: VirtAddr,
        paging_manager: &'a mut PagingManager
    ) -> Option<Process<'a>> {
        let user_offset_page_table = unsafe {
            OffsetPageTable::new(user_page_table.0, paging_manager.mapper.phys_offset())
        };

        let mut user_pm = PagingManager {
            mapper: user_offset_page_table,
            frame_allocator: paging_manager.frame_allocator.clone()
        };



        let stack = Stack::allocate_with(&mut user_pm, 0x0030_0000, 2)?;

        /*user_pm.map_page(
            stack_page,
            stack_frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        );*/

        // heap

        reserve_memory(
            VirtAddr::new(0x0060_0000),
            mem_size as u64,
            &mut user_pm.mapper,
            &mut user_pm.frame_allocator,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        );


        // entry
        reserve_memory(
            VirtAddr::new(0x0050_0000),
            0x0060_0000 - 0x0050_0000,
            &mut user_pm.mapper,
            &mut user_pm.frame_allocator,
            PageTableFlags::PRESENT | PageTableFlags::BIT_9 | PageTableFlags::USER_ACCESSIBLE
        );


        return Some(Process {
            threads: Vec::new(),
            memory: ProcessMemoryContext { paging_manager: user_pm, page_table_addr: user_page_table.1, entry_point, stack },
            ring: Ring::Ring3
        });
    }

    pub unsafe fn execute(&mut self) {
        if self.ring == Ring::Ring0 {
            error!("Can't execute a ring 0 process");
            return;
        }

        Cr3::write(self.memory.page_table_addr, Cr3Flags::empty());
        println_serial!("{:?}", self.memory.paging_manager.mapper.translate(VirtAddr::new(0x301ff7)));

        use x86_64::registers::segmentation::*;
       // CS::set_reg(GDT.1.user_code_selector);
        DS::set_reg(GDT.1.user_data_selector);
        ES::set_reg(GDT.1.user_data_selector);
        FS::set_reg(GDT.1.user_data_selector);
        GS::set_reg(GDT.1.user_data_selector);
        //CS::set_reg(GDT.1.user_code_selector);



        unsafe {
            join_thread(self.memory.stack.stack_top - 1, self.memory.entry_point.as_u64());
        }
        loop {}


    }
}


global_asm!(
    include_str!(
        concat!(
            env!("CARGO_MANIFEST_DIR"), "/asm/switch.asm"
        )
    )
);

unsafe extern "C" {
    fn switch_thread(output: *mut Registers, input: *const Registers);
    fn join_thread(stack: u64, entry: u64);
}
