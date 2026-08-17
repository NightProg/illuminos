use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::time::Duration;
use crate::{
    syscall::{SyscallCtx, SyscallResult},
    thread::{
        sleep,
        SCHEDULER,
        process::{self, ProcessArguments},
    },
};
use crate::syscall::SyscallError;

pub fn sys_getpid_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let scheduler = SCHEDULER.lock();
    if let Some(task) = scheduler.current_task() {
        Ok(Some(task.parent))
    } else {
        Err(super::SyscallError::ProcessNotFound)
    }
}

pub fn sys_fork_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let current = SCHEDULER.lock().current;
    let new_pid = process::fork(current, ctx);
    crate::debug!("NEW PID : {:?}", new_pid);
    if let Some(pid) = new_pid {
        Ok(Some(pid))
    } else {
        Err(super::SyscallError::ForkFailed)
    }
}

pub fn sys_execve_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let path = ctx.rdi;
    let args = ctx.rsi;
    let env = ctx.rdx;
    let cstr = unsafe { core::ffi::CStr::from_ptr(path as *const i8) };
    let cstr_str = cstr.to_string_lossy();
    let argc = unsafe {
        let mut count = 0;
        let mut ptr = args as *const *const i8;
        while !(*ptr).is_null() {
            count += 1;
            ptr = ptr.add(1);
        }
        count
    };
    let argv = unsafe {
        core::slice::from_raw_parts(args as *const *const i8, argc)
            .iter()
            .map(|&arg| {
                let cstr = unsafe { core::ffi::CStr::from_ptr(arg) };
                cstr.to_string_lossy().to_string()
            })
            .collect::<Vec<String>>()
    };
    let envp = unsafe {
        let mut count = 0;
        let mut ptr = env as *const *const i8;
        if ptr.is_null() {
            Vec::new()
        } else {
            while !(*ptr).is_null() {
                count += 1;
                ptr = ptr.add(1);
            }
            core::slice::from_raw_parts(env as *const *const i8, count)
                .iter()
                .map(|&env| {
                    let cstr = unsafe { core::ffi::CStr::from_ptr(env) };
                    cstr.to_string_lossy().to_string()
                })
                .collect::<Vec<String>>()
        }
    };

    let process_argument = ProcessArguments { argv, envp };
    let res = process::execve(&cstr_str, process_argument);
    if let Err(e) = res {
        crate::debug!("execve failed: {:?}", e);
        Err(super::SyscallError::ExecveFailed)
    } else {
        Ok(None)
    }
}

pub fn sys_sleep_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let ms = ctx.rdi;
    let duration = Duration::from_millis(ms);
    sleep(duration).map_err(SyscallError::SleepError)?;
    Ok(None)
}

pub fn sys_thread_create(ctx: &mut SyscallCtx) -> SyscallResult {
    let rip = ctx.rdi;
    let rsp = ctx.rsi;
    let stack_size = ctx.rdx;
    let flags = ctx.r10;


}