use alloc::vec::Vec;
use spin::Mutex;

use crate::{
    sync::ring_buffer::AtomicRingBuffer,
    thread::{SCHEDULER, TaskId, schedule},
};

pub struct WaitQueue {
    waiters: Vec<TaskId>,
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            waiters: Vec::new(),
        }
    }

    pub fn wait(&mut self) {
        let mut scheduler_lock = SCHEDULER.lock();
        let current_task = scheduler_lock.current_task().unwrap();
        self.waiters.push(current_task.id);
        scheduler_lock.block_current_task();
        drop(scheduler_lock);
        schedule();
    }

    pub fn wake(&self, id: TaskId) -> Option<()> {
        let res = self.waiters.iter().find(|i| **i == id)?;
        SCHEDULER.lock().wake_task(*res);
        Some(())
    }

    pub fn wake_all(&self) {
        for waiter in self.waiters.clone() {
            SCHEDULER.lock().wake_task(waiter);
        }
    }
}
pub struct AtomicWaitQueue {
    waiters: AtomicRingBuffer<TaskId, 1024>,
}

impl AtomicWaitQueue {
    pub const fn new() -> Self {
        AtomicWaitQueue {
            waiters: AtomicRingBuffer::new(),
        }
    }
    pub fn wait(&self) {
        let mut scheduler_lock = SCHEDULER.lock();
        let current_task = scheduler_lock.current_task().unwrap();
        self.waiters.push(current_task.id);
        scheduler_lock.block_current_task();
        drop(scheduler_lock);
        schedule();
    }

    pub fn wake_all(&self) {
        for waiter in self.waiters.pop_n(1024) {
            SCHEDULER.lock().wake_task(waiter);
        }
    }
}
