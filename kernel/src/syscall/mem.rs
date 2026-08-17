use crate::{
    allocator::{
        mmap,
        vma::{MapFlags, ProtFlags},
    },
    syscall::{SyscallCtx, SyscallResult},
    thread::{
        SCHEDULER,
        process::{self, PROCESSES},
    },
};

pub fn sys_mmap_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let addr = ctx.rdi;
    let length = ctx.rsi;
    let prot = ctx.rdx;
    let flags = ctx.r10;
    let fd = ctx.r8;
    let offset = ctx.r9 as u64;
    let prot = ProtFlags::from_bits_truncate(prot);
    let flags = MapFlags::from_bits_truncate(flags);
    let mut process_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = process_lock.get_mut(&current_task.parent).unwrap();

    let res = mmap::mmap(process, addr, length, prot, flags, fd, offset);
    match res {
        Ok(mapped_addr) => Ok(Some(mapped_addr)),
        Err(e) => Err(super::SyscallError::MmapFailed),
    }
}

pub fn sys_munmap_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let addr = ctx.rdi;
    let length = ctx.rsi;
    let mut processes_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = processes_lock.get_mut(&current_task.parent).unwrap();
    if let Err(e) = mmap::munmap(process, addr, length) {
        Err(super::SyscallError::MunmapFailed)
    } else {
        Ok(None)
    }
}

pub fn sys_mprotect_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let addr = ctx.rdi;
    let length = ctx.rsi;
    let prot = ctx.rdx;
    let prot = ProtFlags::from_bits_truncate(prot);
    let mut processes_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = processes_lock.get_mut(&current_task.parent).unwrap();
    if let Err(e) = mmap::mprotect(process, addr, length, prot) {
        Err(super::SyscallError::MprotectFailed)
    } else {
        Ok(None)
    }
}

pub fn sys_brk_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let addr = ctx.rdi;
    let mut processes_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = processes_lock.get_mut(&current_task.parent).unwrap();
    if let Err(e) = process::brk(process, addr) {
        Err(super::SyscallError::BrkFailed)
    } else {
        Ok(Some(addr))
    }
}
