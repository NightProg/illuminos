use x86_64::structures::idt::InterruptStackFrame;

use crate::{debug, io::stdout, println_serial};

pub type SignalHandler = u64;

#[derive(Debug, Clone, Copy, Default)]
pub struct SignalState {
    pub pending: u64,
    pub handlers: [SignalHandler; 32],
}

impl SignalState {
    pub const fn new() -> Self {
        Self {
            pending: 0,
            handlers: [0; 32],
        }
    }

    pub fn send(&mut self, sig: Signal) {
        self.pending |= 1 << sig as u64;
    }

    pub fn pop_pending(&mut self) -> Option<Signal> {
        if self.pending == 0 {
            return None;
        }
        let bit = self.pending.trailing_zeros() as u64;
        self.pending &= !(1 << bit);
        Signal::from_u64(bit)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Kill,
    Stop,
    Continue,
    PageFault,
    USR0,
    USR1,
    URS3,
}

impl Signal {
    pub fn from_u64(value: u64) -> Option<Self> {
        Some(match value {
            1 => Signal::Kill,
            2 => Signal::Stop,
            3 => Signal::Continue,
            4 => Signal::PageFault,
            20 => Signal::USR0,
            21 => Signal::USR1,
            22 => Signal::URS3,
            _ => return None,
        })
    }

    pub fn to_u8(&self) -> u64 {
        match self {
            Signal::Kill => 1,
            Signal::Stop => 2,
            Signal::Continue => 3,
            Signal::PageFault => 4,
            Signal::USR0 => 20,
            Signal::USR1 => 21,
            Signal::URS3 => 22,
        }
    }
}
pub fn deliver_signals_irq(frame: &mut InterruptStackFrame) {
    debug!("Delivering signals");
    let (sig, handler) = {
        let mut scheduler = crate::thread::SCHEDULER.lock();
        let task = match scheduler.current_task_mut() {
            Some(t) => t,
            None => return,
        };
        let sig = match task.signals.pop_pending() {
            Some(s) => s,
            None => return,
        };
        let handler = task.signals.handlers[sig as usize];
        (sig, handler)
    };

    debug!("Delivering signal: {:?}, handler={:X}", sig, handler);

    if sig == Signal::Kill {
        crate::thread::task_exit();
    }

    if handler == 0 {
        return;
    }

    let frame_ptr = frame as *const _ as *mut u64;
    unsafe {
        let saved_rip = *frame_ptr.add(0);
        let user_rsp = *frame_ptr.add(3);

        let new_rsp = user_rsp - 8;
        *(new_rsp as *mut u64) = saved_rip; // push sur stack user

        *frame_ptr.add(0) = handler; // rip → handler
        *frame_ptr.add(3) = new_rsp; // rsp → new_rsp
    }
}
