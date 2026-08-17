use alloc::{collections::vec_deque::VecDeque, string::ToString, sync::Arc, vec::Vec};
use pc_keyboard::DecodedKey;
use spin::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{context::GLOBAL_CONTEXT, dbg, drivers::keyboard::KEYBOARD, fs::{Inode, Result}, println_serial, sync::wait_queue::AtomicWaitQueue, thread::process::current_process, tty::session::SessionId};

pub struct TtyInode;

impl Inode for TtyInode {
    fn read_at(&mut self, _offset: u64, buf: &mut [u8]) -> Result<usize> {
        loop {
            let tty_data = {
                let sessions = GLOBAL_CONTEXT.sessions.read();
                let current_session = sessions.current();
                let current_process = current_process().ok_or("No current process".to_string())?;
                let session_id = current_process.current_session.ok_or("Process has no session".to_string())?;

                if session_id != current_session.id {
                    return Err("Cannot read from non-current session".to_string());
                }

                if current_process.pid != current_session.foreground_process {
                    return Err("Process is not in foreground".to_string());
                }

                if current_session.tty.is_ready() {
                    let dbuf = current_session.tty.pop_n(buf.len());
                    dbg!("Read from tty: {:?}", core::str::from_utf8(&dbuf));
                    if !dbuf.is_empty() {
                        Some(dbuf)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(dbuf) = tty_data {
                let size = dbuf.len();
                for n in 0..size {
                    buf[n] = dbuf[n];
                }
                return Ok(size);
            }

            // Wait for data
            {
                let sessions = GLOBAL_CONTEXT.sessions.read();
                sessions.current().tty.wait();
            }
        }
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<usize> {
        let sessions = GLOBAL_CONTEXT.sessions.try_read();
        if sessions.is_none() {
            return Ok(0);
        }

        let mut sessions = sessions.unwrap();
        let current_session = sessions.current();
        let current_process = current_process().unwrap();
        let session_id = current_process.current_session;
        if session_id.is_none() {
            return Err("Cannot write tty".to_string());
        }

        let session_id = session_id.unwrap();
        if session_id != current_session.id {
            return Err("cannot write to tty: no val".to_string());
        }

        if current_process.pid != current_session.foreground_process
            && !current_session
                .background_processes
                .contains(&current_process.pid)
        {
            return Err("cannot write tty".to_string());
        }

        current_session.tty.write_buf(buf);
        Ok(buf.len())
    }

    fn size(&mut self) -> u64 {
        0
    }

    fn kind(&self) -> crate::fs::InodeKind {
        crate::fs::InodeKind::Device
    }

    fn inner_as_any(&mut self) -> &dyn core::any::Any {
        self
    }

    fn inner_as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}
