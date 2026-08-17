use x86_64::{PhysAddr, VirtAddr};

use crate::{allocator::paging::KERNEL_PAGING_MANAGER, drivers::pit::pit_wait_ms};

pub const VECTOR_SPURIOUS: u8 = 0xFF;
pub const VECTOR_TIMER: u8 = 0x20;
pub const VECTOR_KEYBOARD: u8 = 0x21;

const LAPIC_ID: u32 = 0x020;
const LAPIC_VER: u32 = 0x030;
const LAPIC_TPR: u32 = 0x080; // Task Priority Register
const LAPIC_EOI: u32 = 0x0B0; // End Of Interrupt
const LAPIC_SVR: u32 = 0x0F0; // Spurious Interrupt Vector Register
const LAPIC_ICR_LO: u32 = 0x300; // Interrupt Command Register (low)
const LAPIC_ICR_HI: u32 = 0x310; // Interrupt Command Register (high)
const LAPIC_LVT_TIMER: u32 = 0x320;
const LAPIC_TIMER_IC: u32 = 0x380; // Initial Count
const LAPIC_TIMER_CC: u32 = 0x390; // Current Count
const LAPIC_TIMER_DIV: u32 = 0x3E0; // Divide Configuration

const IOAPIC_ID: u32 = 0x00;
const IOAPIC_VER: u32 = 0x01;
const IOAPIC_REDTBL: u32 = 0x10;

const IOAPIC_PHYS_BASE: u64 = 0xFEC0_0000;

const TIMER_MODE_ONESHOT: u32 = 0b00 << 17;
const TIMER_MODE_PERIODIC: u32 = 0b01 << 17;
const TIMER_MASKED: u32 = 1 << 16;

static HHDM_OFFSET: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);

pub fn set_hhdm_offset(offset: u64) {
    HHDM_OFFSET.store(offset, core::sync::atomic::Ordering::Relaxed);
}

pub fn hhdm_offset() -> u64 {
    HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed)
}

fn phys_to_virt(phys: u64) -> *mut u32 {
    (hhdm_offset() + phys) as *mut u32
}

fn lapic_base_phys() -> u64 {
    unsafe {
        let mut msr = x86_64::registers::model_specific::Msr::new(0x1B);
        msr.read() & 0x0000_FFFF_FFFF_F000
    }
}

#[inline]
pub unsafe fn lapic_read(reg: u32) -> u32 {
    let base = phys_to_virt(lapic_base_phys());
    unsafe { core::ptr::read_volatile(base.byte_add(reg as usize)) }
}

#[inline]
unsafe fn lapic_write(reg: u32, val: u32) {
    let base = phys_to_virt(lapic_base_phys());
    unsafe { core::ptr::write_volatile(base.byte_add(reg as usize), val) }
}

#[inline]
pub fn lapic_eoi() {
    unsafe { lapic_write(LAPIC_EOI, 0) }
}

pub unsafe fn init_local_apic() {
    unsafe {
        lapic_write(LAPIC_SVR, (1 << 8) | VECTOR_SPURIOUS as u32);
        lapic_write(LAPIC_TPR, 0);
        lapic_write(LAPIC_LVT_TIMER, 1 << 16);
    }
    crate::println_serial!("Local APIC initialized (base={:#x})", lapic_base_phys());
}

unsafe fn ioapic_read(base: *mut u32, reg: u32) -> u32 {
    unsafe {
        core::ptr::write_volatile(base, reg); // IOREGSEL
        core::ptr::read_volatile(base.byte_add(0x10)) // IOWIN
    }
}

unsafe fn ioapic_write(base: *mut u32, reg: u32, val: u32) {
    unsafe {
        core::ptr::write_volatile(base, reg);
        core::ptr::write_volatile(base.byte_add(0x10), val);
    }
}

fn redtbl_entry(vector: u8, masked: bool, dest_apic_id: u8) -> u64 {
    let lo = vector as u64
        | (0b000 << 8)  // Fixed delivery
        | (0 << 11)     // Physical destination
        | (0 << 13)     // Active high
        | (0 << 15)     // Edge triggered
        | ((masked as u64) << 16);
    let hi = (dest_apic_id as u64) << 56;
    hi | lo
}

pub unsafe fn ioapic_route(irq: u8, vector: u8, dest_lapic: u8, masked: bool) {
    let base = phys_to_virt(IOAPIC_PHYS_BASE);
    let entry = redtbl_entry(vector, masked, dest_lapic);
    let reg_lo = IOAPIC_REDTBL + 2 * irq as u32;
    let reg_hi = reg_lo + 1;
    unsafe {
        ioapic_write(base, reg_hi, (entry >> 32) as u32);
        ioapic_write(base, reg_lo, entry as u32);
    }
}

pub unsafe fn ioapic_mask(irq: u8) {
    let base = phys_to_virt(IOAPIC_PHYS_BASE);
    let reg = IOAPIC_REDTBL + 2 * irq as u32;
    unsafe {
        let lo = ioapic_read(base, reg);
        ioapic_write(base, reg, lo | (1 << 16));
    }
}

pub unsafe fn ioapic_unmask(irq: u8) {
    let base = phys_to_virt(IOAPIC_PHYS_BASE);
    let reg = IOAPIC_REDTBL + 2 * irq as u32;
    unsafe {
        let lo = ioapic_read(base, reg);
        ioapic_write(base, reg, lo & !(1 << 16));
    }
}

pub unsafe fn map_apic_regions() {
    use x86_64::structures::paging::PageTableFlags;

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    let lapic_phys = lapic_base_phys();
    KERNEL_PAGING_MANAGER.lock().as_mut().unwrap().map_memory(
        VirtAddr::new(hhdm_offset() + lapic_phys),
        0x1000, // 4KB suffisent
        PhysAddr::new(lapic_phys),
        flags,
    );

    KERNEL_PAGING_MANAGER.lock().as_mut().unwrap().map_memory(
        VirtAddr::new(hhdm_offset() + IOAPIC_PHYS_BASE),
        0x1000,
        PhysAddr::new(IOAPIC_PHYS_BASE),
        flags,
    );

    crate::println_serial!(
        "APIC MMIO mapped: lapic={:#x} ioapic={:#x}",
        lapic_phys,
        IOAPIC_PHYS_BASE
    );
}

pub unsafe fn calibrate_apic_timer() -> u64 {
    unsafe {
        lapic_write(LAPIC_TIMER_DIV, 0b1011); // divide by 1

        lapic_write(LAPIC_LVT_TIMER, TIMER_MASKED | TIMER_MODE_ONESHOT);

        lapic_write(LAPIC_TIMER_IC, u32::MAX);

        pit_wait_ms(1);

        let remaining = lapic_read(LAPIC_TIMER_CC);
        let elapsed = u32::MAX - remaining;

        let freq = elapsed as u64 * 100;

        crate::println_serial!(
            "APIC timer: {} ticks en 10ms → freqq = {} Hz ({} MHz)",
            elapsed,
            freq,
            freq / 1_000_000
        );

        freq
    }
}

pub unsafe fn init_apic_timer(target_hz: u32, vector: u8) -> u64 {
    unsafe {
        let apic_freq = calibrate_apic_timer();

        let initial_count = apic_freq / target_hz as u64;

        if initial_count > u32::MAX as u64 {
            lapic_write(LAPIC_TIMER_DIV, 0b0011); // divide by 16
            let count = (apic_freq / 16) / target_hz as u64;
            lapic_write(LAPIC_TIMER_IC, count as u32);
        } else {
            lapic_write(LAPIC_TIMER_DIV, 0b1011); // divide by 1
            lapic_write(LAPIC_TIMER_IC, initial_count as u32);
        }

        lapic_write(LAPIC_LVT_TIMER, TIMER_MODE_PERIODIC | vector as u32);

        apic_freq
    }
}
