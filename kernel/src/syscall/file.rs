use alloc::string::ToString;

use crate::{
    fs, println_serial,
    syscall::{SyscallCtx, SyscallResult},
    thread::{
        SCHEDULER,
        process::{self, PROCESSES},
    },
};


pub fn sys_read_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let fd = ctx.rdi;
    let mut buf = ctx.rsi as *mut u8;
    let mut buf_len = ctx.rdx;
    let slice = unsafe { core::slice::from_raw_parts_mut(buf, buf_len as usize) };

    let (task_parent, file) = {
        let scheduler = SCHEDULER.lock();
        let task = scheduler.current_task().cloned().unwrap();
        let parent = task.parent;
        let file = PROCESSES
            .lock()
            .get(&parent)
            .unwrap()
            .open_files
            .get(fd as usize);
        (parent, file)
    };
    if let Some(file) = file {
        let mut file = file.lock();
        match file.read(slice) {
            Ok(read) => Ok(Some(read as u64)),
            Err(e) => Err(super::SyscallError::FileReadError),
        }
    } else {
        Err(super::SyscallError::InvalidFileDescriptor)
    }
}

pub fn sys_write_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let fd = ctx.rdi;
    let buf = ctx.rsi as *const u8;
    let buf_len = ctx.rdx;
    let slice = unsafe { core::slice::from_raw_parts(buf, buf_len as usize) };

    println_serial!("WRITTING: {:?}", core::str::from_utf8(slice));
    println_serial!("CALLING SYS CALL WRITE: fd={}, buf={:?}, len={}", fd, core::str::from_utf8(slice), buf_len);
    let (task_parent, file) = {
        let scheduler = SCHEDULER.lock();
        let task = scheduler.current_task().cloned().unwrap();
        let parent = task.parent;
        let file = PROCESSES
            .lock()
            .get(&parent)
            .unwrap()
            .open_files
            .get(fd as usize);
        (parent, file)
    };

    if let Some(file) = file {
        let mut file = file.lock();
        match file.write(slice) {
            Ok(written) => Ok(Some(written as u64)),
            Err(e) => Err(super::SyscallError::FileWriteError),
        }
    } else {
        Err(super::SyscallError::InvalidFileDescriptor)
    }
}

pub fn sys_openf_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let addr = unsafe { ctx.rdi as *mut i8 };

    let fd_addr = unsafe { ctx.rsi as *mut u64 };

    let flags = fs::OpenFlags::from_bits_truncate(ctx.rdx);

    let cstr = unsafe { core::ffi::CStr::from_ptr(addr) };
    let string = cstr.to_string_lossy().to_string();
    let current_task = crate::thread::SCHEDULER
        .lock()
        .current_task()
        .cloned()
        .unwrap();
    let mut processes_lock = PROCESSES.lock();
    let current_process = processes_lock.get_mut(&current_task.parent).unwrap();
    let file = current_process.open_file_path(&string, flags);
    if file.is_err() {
        return Err(super::SyscallError::FileOpenError);
    }
    let file = file.unwrap();

    Ok(Some(file as u64))
}

pub fn sys_dup2_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let old_fd = ctx.rdi as usize;
    let new_fd = ctx.rsi as usize;
    let file = {
        let processes = PROCESSES.lock();
        let task = SCHEDULER.lock().current_task().cloned().unwrap();
        processes.get(&task.parent).unwrap().open_files.get(old_fd)
    };
    if file.is_none() {
        return Err(super::SyscallError::InvalidFileDescriptor);
    }
    let file = file.unwrap();

    {
        let mut processes = PROCESSES.lock();
        let task = SCHEDULER.lock().current_task().cloned().unwrap();
        let fds = &mut processes.get_mut(&task.parent).unwrap().open_files;
        if fds.insert_at(new_fd, file).is_none() {
            Err(super::SyscallError::InvalidFileDescriptor)
        } else {
            Ok(None)
        }
    }
}

pub fn sys_ioctl_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let fd = ctx.rdi as usize;
    let request = ctx.rsi;
    let argp = ctx.rdx;
    let mut processes_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = processes_lock.get_mut(&current_task.parent).unwrap();
    if process.ioctl(fd, request, argp).is_err() {
        Err(super::SyscallError::IoctlFailed)
    } else {
        Ok(None)
    }
}

pub fn sys_close_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let fd = ctx.rdi as usize;
    let mut processes_lock = PROCESSES.lock();
    let scheduler_lock = SCHEDULER.lock();
    let current_task = scheduler_lock.current_task().cloned().unwrap();
    let process = processes_lock.get_mut(&current_task.parent).unwrap();
    if process.close_file(fd).is_err() {
        Err(super::SyscallError::InvalidFileDescriptor)
    } else {
        Ok(None)
    }
}

pub fn sys_pipe_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let fd_ptr = ctx.rdi as *mut u64;
    let res = process::pipe();
    match res {
        Ok((read_fd, write_fd)) => unsafe {
            *fd_ptr = read_fd as u64;
            *(fd_ptr.add(1)) = write_fd as u64;
            Ok(None)
        },
        Err(_) => Err(super::SyscallError::PipeCreationFailed),
    }
}
