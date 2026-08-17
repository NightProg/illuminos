use crate::{
    info,
    syscall::{SyscallCtx, SyscallResult},
    thread::signal::Signal,
};

pub fn sys_signal_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let signum = ctx.rdi;
    let handler = ctx.rsi;
    info!(
        "Register signal handler: signum={}, handler={:X}",
        signum, handler
    );
    let sig = Signal::from_u64(signum);
    if sig.is_none() {
        return Err(crate::syscall::SyscallError::SignalError);
    }
    let sig = sig.unwrap();

    info!("Parsed signal: {:?}", sig);
    let mut scheduler = crate::thread::SCHEDULER.lock();
    if let Some(task) = scheduler.current_task_mut() {
        if signum < 32 {
            info!(
                "Registering handler for signal {:?} at {:X} for task {}",
                sig, handler, task.id
            );
            task.signals.handlers[signum as usize] = handler;
            Ok(None)
        } else {
            Err(crate::syscall::SyscallError::SignalError)
        }
    } else {
        Err(crate::syscall::SyscallError::NotCurrentProcess)
    }
}

pub fn sys_raise_impl(ctx: &mut SyscallCtx) -> SyscallResult {
    let target_pid = ctx.rdi as usize;
    let signum = ctx.rsi;

    let sig = Signal::from_u64(signum);
    if sig.is_none() {
        return Err(crate::syscall::SyscallError::SignalError);
    }

    let mut scheduler = crate::thread::SCHEDULER.lock();
    let task = scheduler
        .tasks
        .iter_mut()
        .map(|kv| kv.1)
        .find(|t| t.id == target_pid);

    match task {
        Some(t) => {
            t.signals.send(sig.unwrap());
            Ok(None)
        }
        None => Err(crate::syscall::SyscallError::ProcessNotFound),
    }
}
