use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use lazy_static::lazy_static;
use pc_keyboard::KeyEvent;
use spin::Mutex;
use crate::drivers::keyboard::KEYBOARD_STREAM;

pub mod text_buffer;
pub mod console;

lazy_static! {
    pub static ref APPLICATION_MANAGER: Mutex<ApplicationManager> = Mutex::new(ApplicationManager::new());
}


pub trait Application {
    fn handle_event(&mut self, key_event: KeyEvent);
    fn run(&mut self) -> bool;
}


#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ApplicationState {
    Running,
    Stopped,
}
pub struct ApplicationManager {
    applications: Vec<(Box<dyn Application>, ApplicationState)>,
}

unsafe impl Send for ApplicationManager {}
unsafe impl Sync for ApplicationManager {}

impl ApplicationManager {
    pub fn new() -> Self {
        Self {
            applications: Vec::new(),
        }
    }

    pub fn add_application<A: Application + Clone + 'static>(&mut self, app: &A) {
        self.applications.push((Box::new(app.clone()), ApplicationState::Stopped));
    }

    pub fn handle_event(&mut self, key_event: KeyEvent) {
        for app in &mut self.applications {
            app.0.handle_event(key_event.clone());
        }
    }

    /*pub fn run_applications(&mut self) {
        for (app, state) in &mut self.applications {
            let mut running = true;
            while running {
                while let Some(key_event) = KEYBOARD_STREAM.pop() {
                    app.handle_event(key_event);
                }
                running = app.run();
            }
        }
    }*/
}
