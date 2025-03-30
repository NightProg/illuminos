use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use crate::drivers::keyboard::KeyEvent;


lazy_static! {
    pub static ref APP_MANAGER: Mutex<AppManager> = Mutex::new(AppManager::new());
}

pub struct AppManager {
    apps: Vec<Mutex<Box<dyn Application>>>,
    current_app: usize,
}

impl AppManager {
    pub fn new() -> Self {
        AppManager {
            apps: Vec::new(),
            current_app: 0,
        }
    }
    
    pub fn add_app(&mut self, app: Mutex<Box<dyn Application>>) -> usize {
        self.apps.push(
            app
        );
        
        self.current_app = self.apps.len() - 1;
        
        self.apps.len() - 1
    }
    
    pub fn remove_app(&mut self, app_id: usize) {
        self.apps.remove(app_id);
    }
    
    pub fn init(&mut self) {
        for app in &mut self.apps {
            app.lock().window();
        }
    }
    
    pub fn refresh(&mut self) {
        if self.apps.len() > 0 {
            self.apps[self.current_app].lock().refresh();
        }
    }
    
    pub fn handle_keyboard_event(&mut self, event: KeyEvent) {
        if self.apps.len() > 0 {
            self.apps[self.current_app].lock().handle_keyboard_event(event);
        }
    }
    
    
}

pub trait Application: Send + Sync {
    fn window(&mut self) -> usize;

    fn handle_keyboard_event(&mut self, event: KeyEvent);

    fn refresh(&mut self);
} 