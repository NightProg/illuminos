/*use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, KeyCode, KeyEvent};
use crate::drivers::disk::ata::AtaPio;
use crate::drivers::keyboard::KEYBOARD;
use crate::fs::ext2::Ext2FS;
use crate::fs::{Error, FileSystem, Path};
use crate::modern_graphic::app::Application;
use crate::modern_graphic::app::text_buffer::TextBuffer;
use crate::modern_graphic::font::Psf2Font;
use crate::modern_graphic::image::{Image, ImageFormat, Swapchain};
use crate::modern_graphic::queue::Queue;
use crate::modern_graphic::surface::Surface;
use crate::println;

#[derive(Clone)]
pub struct Console {
    swapchain: Swapchain,
    buffer: TextBuffer,
    string_to_draw: String,
    event_queue: Vec<KeyEvent>,
    current_directory: Path,
    current_command: String,
    is_running: bool,
    fs: Ext2FS<AtaPio>
}

impl Console {
    pub fn new(surface: Surface, psf2font: Psf2Font, ext2fs: Ext2FS<AtaPio>) -> Console {
        let queue = Queue::new(true);
        let mut swapchain = Swapchain::new(surface, ImageFormat::BGRA32, 2);
        let dimensions = (800.0, 600.0);
        let binding = swapchain.clone();
        let target = binding.current_image();

        Console {
            swapchain,
            buffer: TextBuffer::new(queue, target.clone(), psf2font, dimensions),
            string_to_draw: String::from("> "),
            event_queue: Vec::new(),
            current_directory: Path::new("/"),
            current_command: String::new(),
            fs: ext2fs,
            is_running: true,
        }
    }

    pub fn flush(&mut self) {
        self.swapchain.next_image();
        let target = self.swapchain.current_image();
        self.swapchain.present(&self.buffer.flush(target.clone()));
    }

    pub fn execute_command(&mut self) {
        let commands = self.current_command.trim().split_whitespace().collect::<Vec<&str>>();

        if commands.is_empty() {
            return;
        }

        if commands[0] == "pwd" {
            self.string_to_draw.push_str(&format!("Current directory: {} ", self.current_directory.to_string()));
        } else if commands[0] == "cd" {

            if commands.len() > 1 {
                let new_path = Path::new(commands[1]);
                if new_path.is_absolute() {
                    self.current_directory = new_path;
                } else {
                    self.current_directory = self.current_directory.join(commands[1]);
                }
                self.string_to_draw.push_str(&format!("Changed directory to: {}", self.current_directory.to_string()));
            } else {
                self.string_to_draw.push_str("Usage: cd <directory>");
            }
        } else if commands[0] == "ls" {
            self.string_to_draw.push_str(&format!("Listing directory: {}\n", self.current_directory.to_string()));
            let dir = self.fs.read_directory_lazy(self.current_directory.clone());
            match dir {
                Ok(dir) => {
                    let mut n = 0;
                    for entry in dir {
                        if n > 10 {
                            self.string_to_draw.push_str("\n... (more entries not shown)");
                            break;
                        }
                        self.string_to_draw.push_str(entry.name());
                        self.string_to_draw.push_str(" ");
                        n += 1;
                    }


                },
                Err(err) => {
                    match err {
                        Error::FileNotFound => {
                            self.string_to_draw.push_str(&format!("Directory not found: {}", self.current_directory.to_string()));
                        },
                        Error::NotADirectory => {
                            self.string_to_draw.push_str(&format!("Not a directory: {}", self.current_directory.to_string()));
                        },
                        Error::KernelError(err) =>{
                            self.string_to_draw.push_str(&format!("Kernel error: {}", err));
                        },
                        Error::CantUseRelPath => {
                            self.string_to_draw.push_str(&format!("Can't use relative path: {}", self.current_directory.to_string()));
                        },
                        _ => {
                            self.string_to_draw.push_str("A error occurs");
                        }
                    }
                }
            }
        } else if commands[0] == "exit" {
            self.is_running = false;
        } else {
            self.string_to_draw.push_str(&format!("Unknown command: {}", commands[0]));
        }

        self.current_command.clear();
    }

    pub fn draw_string(&mut self, string: &str) {
        self.buffer.draw_string(string);
    }

}

impl Application for Console {
    fn handle_event(&mut self, key_event: KeyEvent) {
        self.event_queue.push(key_event);
    }


    fn run(&mut self) -> bool {
        while let Some(key_event) = self.event_queue.pop() {
            if key_event.state != pc_keyboard::KeyState::Up {
                match key_event.code {
                    KeyCode::A => {
                        self.current_command.push('a');
                        self.string_to_draw.push('a');
                    }
                    KeyCode::B => {
                        self.string_to_draw.push('b');
                        self.current_command.push('b');
                    }
                    KeyCode::C => {
                        self.string_to_draw.push('c');
                        self.current_command.push('c');
                    }
                    KeyCode::D => {
                        self.string_to_draw.push('d');
                        self.current_command.push('d');
                    },
                    KeyCode::E => {
                        self.string_to_draw.push('e');
                        self.current_command.push('e');
                    }
                    KeyCode::F => {
                        self.string_to_draw.push('f');
                        self.current_command.push('f');
                    }
                    KeyCode::G => {
                        self.string_to_draw.push('g');
                        self.current_command.push('g');
                    }
                    KeyCode::H => {
                        self.string_to_draw.push('h');
                        self.current_command.push('h');
                    }
                    KeyCode::I => {
                        self.string_to_draw.push('i');
                        self.current_command.push('i');
                    }
                    KeyCode::J => {
                        self.string_to_draw.push('j');
                        self.current_command.push('j');
                    }
                    KeyCode::K => {
                        self.string_to_draw.push('k');
                        self.current_command.push('k');
                    }
                    KeyCode::L => {
                        self.string_to_draw.push('l');
                        self.current_command.push('l');
                    }
                    KeyCode::M => {
                        self.string_to_draw.push('m');
                        self.current_command.push('m');
                    }
                    KeyCode::N => {
                        self.string_to_draw.push('n');
                        self.current_command.push('n');
                    }
                    KeyCode::O => {
                        self.string_to_draw.push('o');
                        self.current_command.push('o');
                    }
                    KeyCode::P => {
                        self.string_to_draw.push('p');
                        self.current_command.push('p');
                    }
                    KeyCode::Q => {
                        self.string_to_draw.push('q');
                        self.current_command.push('q');
                    }
                    KeyCode::R => {
                        self.string_to_draw.push('r');
                        self.current_command.push('r');
                    }
                    KeyCode::S => {
                        self.string_to_draw.push('s');
                        self.current_command.push('s');
                    }
                    KeyCode::T => {
                        self.string_to_draw.push('t');
                        self.current_command.push('t');
                    }
                    KeyCode::U => {
                        self.string_to_draw.push('u');
                        self.current_command.push('u');
                    }
                    KeyCode::V => {
                        self.string_to_draw.push('v');
                        self.current_command.push('v');
                    }
                    KeyCode::W => {
                        self.string_to_draw.push('w');
                        self.current_command.push('w');
                    }
                    KeyCode::X => {
                        self.string_to_draw.push('x');
                        self.current_command.push('x');
                    }
                    KeyCode::Y => {
                        self.string_to_draw.push('y');
                        self.current_command.push('y');
                    }
                    KeyCode::Z => {
                        self.string_to_draw.push('z');
                        self.current_command.push('z');
                    },
                    KeyCode::Spacebar => {
                        self.string_to_draw.push(' ');
                        self.current_command.push(' ');
                    }
                    KeyCode::Backspace => {
                        if self.string_to_draw.chars().last() != Some('>') {
                            self.string_to_draw.pop();
                            if !self.current_command.is_empty() {
                                self.current_command.pop();
                            }

                        }
                    }
                    KeyCode::Return => {
                        self.string_to_draw.push('\n');
                        self.execute_command();
                        self.string_to_draw.push_str("\n> ");
                    },
                    _ => {
                        println!("{:?}", key_event);
                    }
                }
            }
        }
        let draw_string = self.string_to_draw.clone();
        self.draw_string(&draw_string);
        self.flush();
        self.is_running
    }

}

*/
