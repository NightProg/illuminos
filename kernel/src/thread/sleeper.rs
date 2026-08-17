use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::println;
use crate::thread::{TaskId, SCHEDULER};

pub static SLEEPER_QUEUE: Mutex<SleeperQueue> = Mutex::new(SleeperQueue::new());
#[derive(Copy, Clone)]
pub struct Sleeper {
    task_id: TaskId,
    timeout: u64,
}

pub struct SleeperQueue {
    queue: Vec<Sleeper>,
}

impl SleeperQueue {
    pub const fn new() -> Self {
        SleeperQueue {
            queue: Vec::new(),
        }
    }
    
    pub fn add(&mut self, task_id: TaskId, timeout: u64) {
        self.queue.push(Sleeper {
            task_id,
            timeout,
        });
    }

    pub fn wake_up(&mut self, task_id: TaskId) {
        if !self.queue.iter().any(|s| s.task_id == task_id) {
            return;
        }
        self.queue.retain(|s| s.task_id != task_id);
        SCHEDULER.lock().wake_task(task_id);
    }
    
    pub fn tick(&mut self, current_tick: u64) {
        let mut i = 0;
        while i < self.queue.len() {
            let sleeper = &self.queue[i];
            if sleeper.timeout <= current_tick {
                self.wake_up(sleeper.task_id);
            } else {
                i += 1;
            }
        }
    }
}
