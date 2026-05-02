use crate::{
    allocator::{paging::KERNEL_PAGING_MANAGER, process_paging::ProcessPageTable},
    gdt::KERNEL_STACK_SIZE,
};
use alloc::vec::Vec;
use core::{hint::unreachable_unchecked, sync::atomic::AtomicU64};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, PageTable, PageTableFlags, PhysFrame, Size4KiB,
    },
};

use crate::thread::{SCHEDULER, Scheduler};

pub static PID: AtomicU64 = AtomicU64::new(1);

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FF00_0000;
pub const USER_STACK_SIZE: u64 = 0x8000;

pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

pub struct Process {
    pub pid: u64,
    pub pml4_table: ProcessPageTable,
    pub state: ProcessState,
    pub tasks: Vec<usize>,
}

impl Process {
    pub fn kernel() -> Self {
        Process {
            pid: PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            pml4_table: ProcessPageTable::kernel(),
            state: ProcessState::Ready,
            tasks: Vec::new(),
        }
    }
    
    pub fn new(pml4_table: ProcessPageTable) -> Self {
        Process {
            pid: PID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            pml4_table,
            state: ProcessState::Ready,
            tasks: Vec::new(),
        }
    }

    /*pub unsafe fn exec(&self) -> ! {
        unsafe {
            let (kernel_frame, _) = Cr3::read();
            let mut paging_manager_lock = KERNEL_PAGING_MANAGER.lock();
            let phys_offset = paging_manager_lock
                .as_ref()
                .expect("KERNEL_PAGING_MANAGER not initialized before exec")
                .mapper
                .phys_offset();
            drop(paging_manager_lock); // Release lock before possible prints or context switch

            let rip: u64;
            core::arch::asm!("lea {}, [rip]", out(reg) rip);
            crate::println_serial!("exec RIP = {:#x}", rip);

            // PML4 index de cette adresse
            let pml4_idx = (rip >> 39) & 0x1FF;
            crate::println_serial!("PML4 index = {}", pml4_idx);

            // Vérifier aussi le nouveau PML4
            let new_pml4_virt = phys_offset + self.pml4_frame.start_address().as_u64();
            let new_pml4: &PageTable = unsafe { &*(new_pml4_virt.as_ptr()) };
            for i in 256..512 {
                if !new_pml4[i].is_unused() {
                    crate::println_serial!("new PML4[{}] = {:?}", i, new_pml4[i]);
                }
            }
            crate::println_serial!(
                "(1) iretq → rip={:#x} rsp={:#x} cr3={:#x}",
                self.rip.as_u64(),
                self.rsp.as_u64(),
                self.pml4_frame.start_address().as_u64()
            );
            Cr3::write(self.pml4_frame, Cr3Flags::empty());
            crate::println_serial!(
                "(2) iretq → rip={:#x} rsp={:#x} cr3={:#x}",
                self.rip.as_u64(),
                self.rsp.as_u64(),
                self.pml4_frame.start_address().as_u64()
            );
            jump_to_user(self.rip, self.rsp);
        }
    }*/

    pub fn spawn_user_thread(&mut self, entry_point: VirtAddr) -> usize {
        let stack_base = VirtAddr::new(USER_STACK_TOP - self.tasks.len() as u64 * USER_STACK_SIZE);
        self.pml4_table.map_range(
            stack_base,
            USER_STACK_SIZE,
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE,
            &mut KERNEL_PAGING_MANAGER
                .lock()
                .as_mut()
                .expect("KERNEL_PAGING_MANAGER not initialized before spawn_thread")
                .frame_allocator,
        );

        let stack_top = stack_base + USER_STACK_SIZE;
        self.add_to_scheduler(entry_point, stack_top)
    }

    pub fn add_to_scheduler(&mut self, rip: VirtAddr, rsp: VirtAddr) -> usize {
        let task_id = SCHEDULER.lock().add_user_task(
            rip.as_u64(),
            rsp.as_u64(),
            self.pml4_table.pml4_frame.start_address().as_u64(),
            KERNEL_STACK_SIZE,
        );

        self.tasks.push(task_id);
        task_id
    }
}
unsafe fn jump_to_user(rip: VirtAddr, rsp: VirtAddr) -> ! {
    const USER_CODE_SEGMENT: u64 = 0x20 | 3; // Segment 4, RPL 3
    const USER_DATA_SEGMENT: u64 = 0x18 | 3; // Segment 3, RPL 3
    const RFLAGS_IF: u64 = 0x202;

    unsafe {
        core::arch::asm!(
            // Pousser le frame iretq avec des registres fixes
            "push {ds}",       // ss
            "push r10",        // rsp user
            "push {rflags}",   // rflags
            "push {cs}",       // cs
            "push r11",        // rip
            // Changer les segments après (ax est libre)
            "mov ax, {ds}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "iretq",
            ds     = const USER_DATA_SEGMENT,
            cs     = const USER_CODE_SEGMENT,
            rflags = const RFLAGS_IF,
            in("r10") rsp.as_u64(),
            in("r11") rip.as_u64(),
            options(noreturn)
        );
    }
}
