use core::sync::atomic::AtomicU64;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use pc_keyboard::KeyEvent;

use crate::thread::process::Pid;

static ATOMIC_SYSTEM_ID: AtomicU64 = AtomicU64::new(1);

pub type SessionId = u64;

pub struct Session {
    pub id: SessionId,
    pub tty: super::Tty,
    pub background_processes: Vec<Pid>,
    pub foreground_process: Pid,
}

pub struct Sessions {
    sessions: BTreeMap<SessionId, Session>,
    pub current: SessionId,
}

impl Sessions {
    pub const fn new() -> Self {
        Sessions {
            sessions: BTreeMap::new(),
            current: 0,
        }
    }

    pub fn create_session(&mut self, fg: Pid) -> SessionId {
        let session = Session {
            id: ATOMIC_SYSTEM_ID.load(core::sync::atomic::Ordering::SeqCst),
            tty: super::Tty::new(super::Mode::Cooked),
            background_processes: Vec::new(),
            foreground_process: fg,
        };

        if self.current == 0 {
            self.current = session.id;
        }

        self.sessions.insert(session.id, session);

        self.current
    }

    pub fn current(&self) -> &Session {
        self.sessions.get(&self.current).unwrap()
    }

    pub fn current_mut(&mut self) -> &mut Session {
        self.sessions.get_mut(&self.current).unwrap()
    }
}
