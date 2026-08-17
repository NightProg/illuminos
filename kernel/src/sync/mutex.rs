use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use alloc::{string::String, vec::Vec};

use crate::{debug::StackFrame, println_serial};

pub trait Mutex<T> {
    type MutexGuard<'a>: Deref<Target = T> + DerefMut<Target = T>
    where
        Self: 'a;

    fn lock(&self) -> Self::MutexGuard<'_>;

    fn try_lock(&self) -> Option<Self::MutexGuard<'_>>;
}

impl<T> Mutex<T> for spin::Mutex<T> {
    type MutexGuard<'a>
        = spin::MutexGuard<'a, T>
    where
        spin::Mutex<T>: 'a;

    fn lock(&self) -> Self::MutexGuard<'_> {
        spin::Mutex::lock(self)
    }

    fn try_lock(&self) -> Option<Self::MutexGuard<'_>> {
        spin::Mutex::try_lock(self)
    }
}

impl<T> Mutex<T> for TimeoutMutex<T> {
    type MutexGuard<'a>
        = TimeoutMutexGuard<'a, T>
    where
        Self: 'a;

    fn lock(&self) -> Self::MutexGuard<'_> {
        TimeoutMutex::lock(self)
    }

    fn try_lock(&self) -> Option<Self::MutexGuard<'_>> {
        TimeoutMutex::try_lock(self)
    }
}

pub const TIMEOUT: u64 = 10_000_000;

pub struct TimeoutMutex<T> {
    inner: UnsafeCell<T>,
    lock: AtomicBool,
    stack_trace_owner: AtomicU64,
}

unsafe impl<T: Send> Send for TimeoutMutex<T> {}
unsafe impl<T: Sync> Sync for TimeoutMutex<T> {}

pub struct TimeoutMutexGuard<'a, T> {
    lock: &'a AtomicBool,
    data: *mut T,
}

impl<T> TimeoutMutex<T> {
    pub const fn new(data: T) -> Self {
        TimeoutMutex {
            inner: UnsafeCell::new(data),
            lock: AtomicBool::new(false),
            stack_trace_owner: AtomicU64::new(0),
        }
    }
    pub fn into_inner(self) -> T {
        let TimeoutMutex {
            inner,
            lock,
            stack_trace_owner,
        } = self;

        inner.into_inner()
    }

    pub fn try_lock(&self) -> Option<TimeoutMutexGuard<'_, T>> {
        if self
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            self.stack_trace_owner
                .store(unsafe { StackFrame::rbp() }, Ordering::SeqCst);
            Some(TimeoutMutexGuard {
                lock: &self.lock,
                data: unsafe { &mut *self.inner.get() },
            })
        } else {
            None
        }
    }

    pub fn lock(&self) -> TimeoutMutexGuard<'_, T> {
        let mut timeout: u64 = 0;
        while timeout < TIMEOUT {
            if let Some(guard) = self.try_lock() {
                return guard;
            }

            timeout += 1;
            core::hint::spin_loop();
        }

        let stack_frame_owner = self.stack_trace_owner.load(Ordering::SeqCst);

        if stack_frame_owner == 0 {
            panic!("DEAD LOCK");
        } else {
            use core::fmt::Write;

            println_serial!("{}", stack_frame_owner);

            panic!("DEAD LOCK: \n");
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for TimeoutMutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.try_lock() {
            Some(lock) => write!(f, "TimeoutMutex {{ data: ")
                .and_then(|_| (*lock).fmt(f))
                .and_then(|_| write!(f, " }}")),
            None => write!(f, "TimeoutMutex {{ <loked> }}"),
        }
    }
}

impl<T: Default> Default for TimeoutMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for TimeoutMutexGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: core::fmt::Display> core::fmt::Display for TimeoutMutexGuard<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&**self, f)
    }
}

unsafe impl<T: Send> Send for TimeoutMutexGuard<'_, T> {}
unsafe impl<T: Sync> Sync for TimeoutMutexGuard<'_, T> {}

impl<T> Deref for TimeoutMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for TimeoutMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<T> Drop for TimeoutMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

pub struct IrqSafeMutex<T> {
    inner: spin::Mutex<T>,
}

pub struct IrqSafeMutexGuard<'a, T> {
    guard: spin::MutexGuard<'a, T>,
    was_interrupts_enabled: bool,
}

impl<T> IrqSafeMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            inner: spin::Mutex::new(data),
        }
    }

    pub fn lock(&self) -> IrqSafeMutexGuard<'_, T> {
        let was_interrupts_enabled = x86_64::instructions::interrupts::are_enabled();
        // SAFETY: We are disabling interrupts to ensure that the lock is not preempted by an interrupt handler.
        unsafe { x86_64::instructions::interrupts::disable() };
        IrqSafeMutexGuard {
            guard: self.inner.lock(),
            was_interrupts_enabled,
        }
    }

    pub unsafe fn force_unlock(&self) {
        self.inner.force_unlock();
    }
}

impl<T> Drop for IrqSafeMutexGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: We are re-enabling interrupts when the guard is dropped, which is safe because we are no longer holding the lock.
        if self.was_interrupts_enabled {
            unsafe { x86_64::instructions::interrupts::enable() };
        }
    }
}

impl<T> core::ops::Deref for IrqSafeMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.guard
    }
}

impl<T> core::ops::DerefMut for IrqSafeMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.guard
    }
}
