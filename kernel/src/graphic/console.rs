use crate::context::GLOBAL_CONTEXT;
use crate::drivers::keyboard::KEYBOARD;
use crate::elf::ElfProcess;
use crate::graphic::Color;
use crate::graphic::font::{FONT_DEFAULT, PsfFont};
use crate::graphic::framebuffer::FrameBuffer;
use crate::graphic::text_buffer::TextBuffer;
use crate::io::port::Fd;
use crate::io::{stdin, stdout};
use crate::thread::process::Process;
use crate::thread::{SCHEDULER, yield_now};
use crate::{println, thread};
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::ops::DerefMut;
use core::sync::atomic::{AtomicBool, Ordering};
use pc_keyboard::{KeyCode, KeyEvent, ScancodeSet2};
use spin::Mutex;

pub struct Console<'a, F: FrameBuffer, P: PsfFont> {
    text_buffer: TextBuffer<'a, F, P>,
    current_command: String,
    current_dir: String,
    command_history: Vec<String>,
    command_index: usize,
    saved_command: String,
    is_task_running: Arc<AtomicBool>,
    was_task_running: bool,
}

impl<'a, F: FrameBuffer, P: PsfFont> Console<'a, F, P> {
    pub fn new(framebuffer: &'a mut F, font: &'a P) -> Self {
        let mut text_buffer = TextBuffer::new(framebuffer, font, Color::white(), Color::black());
        text_buffer.put_str(
            r#"
      *        .       *       .       *
+IIII+  +LL----+  +LL----+  +UU--UU+  +MM--MM+  +II+  +NN--NN+  +OO+  +SSSS+
 |II|   |LL     | |LL     | |UU  UU|  |MMMMMM|   |II|  |NNN NN| |O  O| |S   *
  II    |LL     | |LL     | |UU  UU|  |MMMM M|    II   |NN NNN| |O  O| |SSS
  II    |LL     | |LL     | |UU  UU|  |MM  MM|    II   |NN  NN| |O  O|    S|
 |II|   |LL     | |LL     | |UU  UU|  |MM  MM|   |II|  |NN  NN| |O  O|    S|
+IIII+  +LLLLLL+  +LLLLLL+  +UUUUUU+  +MM--MM+  +II+  +NN--NN+  +OO+  +SSSS+
   .        *        .        *        .        *        .

        *         +        .        +         *
type 'help' for a list of commands
"#,
        );
        text_buffer.put_str("> ");

        Self {
            text_buffer,
            current_command: String::new(),
            current_dir: String::from(""),
            command_history: Vec::new(),
            command_index: 0,
            saved_command: String::new(),
            is_task_running: Arc::new(AtomicBool::new(false)),
            was_task_running: false,
        }
    }

    pub fn write_str(&mut self, s: &str) {
        self.text_buffer.put_str(s);
    }

    pub fn clear(&mut self) {
        self.text_buffer.clear();
    }

    pub fn handle_command(&mut self) {
        if self.current_command.is_empty() {
            return;
        }

        let commands = self
            .current_command
            .split_whitespace()
            .collect::<Vec<&str>>();

        match commands[0] {
            "help" => {
                self.write_str("Available commands:\n");
                self.write_str("help - Show this help message\n");
                self.write_str("clear - Clear the console\n");
                self.write_str("echo <message> - Print the message to the console\n");
                self.write_str("ecd - Echo current directory\n");
                self.write_str("scd <path> - Set current directory\n");
                self.write_str("lscd - List files in current directory\n");
                self.write_str("readf <filename> - Read and display file contents\n");
                self.write_str("writef <filename> <content> - Write content to file\n");
                self.write_str("mount <fs> [disk] <mount_point> - Mount a filesystem\n");
                self.write_str("umount <mount_point> - Unmount a filesystem\n");
                self.write_str("mkdir <directory_name> - Create a new directory\n");
                self.write_str("createf <file_name> - Create a new file\n");
                self.write_str("exec <executable> - Execute a program\n");
            }
            "exec" => {
                if commands.len() < 2 {
                    self.write_str("Usage: exec <executable>\n");
                    return;
                }
                let executable = commands[1];
                let inode = unsafe { GLOBAL_CONTEXT.fs.lookup(executable) };
                match inode {
                    Ok(inode) => {
                        let size = inode.lock().size();
                        let mut buf = alloc::vec![0; size as usize];
                        inode.lock().read_at(0, &mut buf).unwrap();
                        let mut elf = ElfProcess::new(&buf);
                        if let Some(rip) = elf.load() {
                            let process = Process::create(*elf.get_page_table());

                            let task = thread::process::spawn(process, rip).unwrap();
                            self.is_task_running.store(true, Ordering::SeqCst);
                            let is_running_clone = self.is_task_running.clone();
                            thread::process::wait_all();
                            is_running_clone.store(false, Ordering::SeqCst);
                            /*SCHEDULER.lock().register_exit_callback(
                                task,
                                Box::new(move || {
                                    is_running_clone.store(false, Ordering::SeqCst);
                                }),
                            );*/
                        } else {
                            self.write_str(
                                "Error: Failed to load ELF (paging manager not initialized? or malformed ELF)\n",
                            );
                        }
                    }
                    Err(e) => {
                        self.write_str(&format!("Error executing file: {:?}\n", e));
                    }
                }
            }
            "clear" => {
                self.clear();
            }
            "echo" => {
                let echo_message = commands[1..].join(" ");
                self.write_str(&format!("{}\n", echo_message));
            }
            "ecd" => {
                self.write_str(&format!("Current directory: {}\n", self.current_dir));
            }
            "scd" => {
                if commands.len() > 1 {
                    let mut dir = commands[1];
                    let sdir = format!("{}/{}", self.current_dir, dir);
                    if dir.as_bytes()[0] != b'/' {
                        dir = &sdir;
                    }
                    self.current_dir = dir.to_string();
                    self.write_str(&format!("Directory changed to: {}\n", self.current_dir));
                } else {
                    self.write_str("Usage: scd <path>\n");
                }
            }
            "lscd" => {
                let entries = unsafe { GLOBAL_CONTEXT.fs.list_entries(&*self.current_dir) };
                match entries {
                    Ok(files) => {
                        self.write_str("Files in current directory:\n");
                        for file in files {
                            self.write_str(&format!("{}\n", file));
                        }
                    }
                    Err(e) => {
                        self.write_str(&format!("Error listing directory: {:?}\n", e));
                    }
                }
            }
            "mount" => {
                // syntax: mount <fs_type> [device] <mount_point>
                if commands.len() > 1 {
                    let fs_type = commands[1];
                    match fs_type {
                        "illfs" => {
                            if commands.len() < 4 {
                                self.write_str("Usage: mount illfs <device> <mount_point>\n");
                                return;
                            }
                            let mut device = commands[2];
                            let sdevice = format!("{}/{}", self.current_dir, device);
                            if device.as_bytes()[0] != b'/' {
                                device = &sdevice;
                            }
                            let mut mount_point = commands[3];
                            let smount_point = format!("{}/{}", self.current_dir, mount_point);

                            if mount_point.as_bytes()[0] != b'/' {
                                mount_point = &smount_point;
                            }
                            let open_file = unsafe {
                                GLOBAL_CONTEXT.fs.open_file(
                                    device,
                                    crate::fs::OpenFlags::READ & crate::fs::OpenFlags::WRITE,
                                )
                            };
                            match open_file {
                                Ok(file) => {
                                    let raw_fs = illfs::IllFs::mount(file);
                                    if let Err(e) = raw_fs {
                                        self.write_str(&format!("Error mounting illfs: {:?}\n", e));
                                        return;
                                    }
                                    let fs = crate::fs::illfs::IllFS(Arc::new(Mutex::new(
                                        raw_fs.unwrap(),
                                    )));

                                    unsafe {
                                        GLOBAL_CONTEXT.fs.mounts.push(crate::fs::Mount {
                                            fs: Arc::new(Mutex::new(fs)),
                                            path: mount_point.to_string(),
                                        });
                                    }
                                }
                                Err(e) => {
                                    self.write_str(&format!(
                                        "Error opening device file: {:?}\n",
                                        e
                                    ));
                                }
                            }
                        }
                        "ramfs" => {
                            if commands.len() < 3 {
                                self.write_str("Usage: mount ramfs <mount_point>\n");
                                return;
                            }
                            let mut mount_point = commands[2];
                            let smount_point = format!("{}/{}", self.current_dir, mount_point);
                            if mount_point.as_bytes()[0] != b'/' {
                                mount_point = &smount_point;
                            }
                            let fs = crate::fs::ramfs::RamFs::new();
                            unsafe {
                                GLOBAL_CONTEXT.fs.mounts.push(crate::fs::Mount {
                                    fs: Arc::new(Mutex::new(fs)),
                                    path: mount_point.to_string(),
                                });
                            }
                        }
                        _ => {
                            self.write_str(
                                "Unsupported filesystem type. Supported types: illfs, ramfs\n",
                            );
                        }
                    }
                } else {
                    self.write_str("Usage: mount <fs> [disk] <mount_point>\n");
                }
            }
            "readf" => {
                // syntax: readf <number of bytes or 'all'> <filename>
                if commands.len() > 2 {
                    let filename = commands[2];
                    let num_bytes_str = commands[1];
                    let mut path = filename;
                    let current_path = format!("{}/{}", self.current_dir, filename);
                    if path.as_bytes()[0] != b'/' {
                        path = &current_path;
                    }
                    let read_inode = unsafe { GLOBAL_CONTEXT.fs.lookup(path) };

                    match read_inode {
                        Ok(inode) => {
                            let mut lock = inode.lock();
                            let size = lock.size() as usize;
                            let to_read = if num_bytes_str == "all" {
                                size
                            } else {
                                num_bytes_str.parse::<usize>().unwrap_or(size)
                            }
                            .min(size);
                            let mut buffer = vec![0u8; to_read];
                            let read_result = lock.read_at(0, &mut buffer);
                            match read_result {
                                Ok(bytes_read) => {
                                    self.write_str(&format!(
                                        "File contents ({} bytes):\n",
                                        bytes_read
                                    ));
                                    let s = buffer[..bytes_read]
                                        .iter()
                                        .map(|b| format!("{:02x}", b))
                                        .collect::<Vec<_>>()
                                        .join(" ");
                                    self.write_str(&s);
                                    self.write_str("\n");
                                }
                                Err(e) => {
                                    self.write_str(&format!("Error reading file: {:?}\n", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.write_str(&format!("Error opening file: {:?}\n", e));
                        }
                    }
                } else {
                    self.write_str("Usage: readf <number of bytes or 'all'> <filename>\n");
                }
            }
            "writef" => {
                // syntax: writef <filename> <content>
                if commands.len() > 2 {
                    let filename = commands[1];
                    let content = commands[2..].join(" ");
                    let mut path = filename;
                    let current_path = format!("{}/{}", self.current_dir, filename);
                    if path.as_bytes()[0] != b'/' {
                        path = &current_path;
                    }
                    let write_inode = unsafe { GLOBAL_CONTEXT.fs.lookup(path) };

                    match write_inode {
                        Ok(inode) => {
                            let mut lock = inode.lock();
                            let write_result = lock.write_at(0, content.as_bytes());
                            match write_result {
                                Ok(bytes_written) => {
                                    self.write_str(&format!(
                                        "Wrote {} bytes to file.\n",
                                        bytes_written
                                    ));
                                }
                                Err(e) => {
                                    self.write_str(&format!("Error writing to file: {:?}\n", e));
                                }
                            }
                        }
                        Err(e) => {
                            self.write_str(&format!("Error opening file: {:?}\n", e));
                        }
                    }
                } else {
                    self.write_str("Usage: writef <filename> <content>\n");
                }
            }
            "mkdir" => {
                if commands.len() > 1 {
                    let dir_name = commands[1];
                    let mut path = dir_name;
                    let current_path = format!("{}/{}", self.current_dir, dir_name);
                    if path.as_bytes()[0] != b'/' {
                        path = &current_path;
                    }

                    println!("Creating directory at path: {}", path);
                    let result = unsafe { GLOBAL_CONTEXT.fs.mkdir(path) };
                    match result {
                        Ok(_) => {
                            self.write_str(&format!("Directory '{}' created.\n", path));
                        }
                        Err(e) => {
                            self.write_str(&format!("Error creating directory: {:?}\n", e));
                        }
                    }
                } else {
                    self.write_str("Usage: mkdir <directory_name>\n");
                }
            }
            "createf" => {
                if commands.len() > 1 {
                    let file_name = commands[1];
                    let mut path = file_name;
                    let current_path = format!("{}/{}", self.current_dir, file_name);
                    if path.as_bytes()[0] != b'/' {
                        path = &current_path;
                    }
                    let result = unsafe { GLOBAL_CONTEXT.fs.create_file(path) };
                    match result {
                        Ok(_) => {
                            self.write_str(&format!("File '{}' created.\n", path));
                        }
                        Err(e) => {
                            self.write_str(&format!("Error creating file: {:?}\n", e));
                        }
                    }
                } else {
                    self.write_str("Usage: createf <file_name>\n");
                }
            }
            "umount" => {
                if commands.len() > 1 {
                    let mount_point = commands[1];
                    let result = unsafe { GLOBAL_CONTEXT.fs.umount(mount_point) };
                    match result {
                        Ok(_) => {
                            self.write_str(&format!("Unmounted '{}'.\n", mount_point));
                        }
                        Err(e) => {
                            self.write_str(&format!("Error unmounting: {:?}\n", e));
                        }
                    }
                } else {
                    self.write_str("Usage: umount <mount_point>\n");
                }
            }
            _ => {
                self.write_str("Unknown command. Type 'help' for a list of commands.\n");
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        use pc_keyboard::DecodedKey;
        let decoded = x86_64::instructions::interrupts::without_interrupts(|| {
            let mut keyboard = unsafe { &mut KEYBOARD };
            keyboard.process_keyevent(key)
        });
        if let Some(key) = decoded {
            match key {
                DecodedKey::Unicode(c) => {
                    if c == '\n' {
                        let is_task_running = self.is_task_running.load(Ordering::SeqCst);
                        if !is_task_running {
                            self.text_buffer.put_char('\n');
                            self.handle_command();
                            self.command_history.push(self.current_command.clone());
                            self.command_index = self.command_history.len();
                            self.current_command.clear();
                            let is_still_running = self.is_task_running.load(Ordering::SeqCst);
                            if !is_still_running {
                                stdout::flush(self);
                                self.text_buffer.put_str("> ");
                            } else {
                                self.was_task_running = true;
                            }
                        } else {
                            stdin::write(b"\n");
                        }
                    } else if c == char::from_u32(8).unwrap() {
                        let is_task_running = self.is_task_running.load(Ordering::SeqCst);
                        if is_task_running {
                            let _ = stdin::pop();
                        } else {
                            if !self.current_command.is_empty() {
                                self.current_command.pop();
                                self.text_buffer.backspace();
                            }
                        }
                    } else {
                        let is_task_running = self.is_task_running.load(Ordering::SeqCst);
                        if is_task_running {
                            stdin::write(&[c as u8]);
                        } else {
                            self.current_command.push(c);
                            self.text_buffer.put_char(c);
                        }
                    }
                }
                DecodedKey::RawKey(KeyCode::ArrowUp) => {
                    if self.command_index == self.command_history.len() {
                        self.saved_command = self.current_command.clone();
                    }

                    if self.command_index > 0 {
                        self.command_index -= 1;

                        for _ in 0..self.current_command.len() {
                            self.text_buffer.backspace();
                        }

                        self.current_command = self.command_history[self.command_index].clone();
                        self.text_buffer.put_str(&self.current_command);
                    }
                }
                DecodedKey::RawKey(KeyCode::ArrowDown) => {
                    if let Some(last_command) =
                        self.command_history.get(self.command_index.wrapping_add(1))
                    {
                        self.command_index += 1;
                        // Clear current line
                        for _ in 0..self.current_command.len() {
                            self.text_buffer.backspace();
                        }
                        self.current_command = last_command.clone();
                        self.text_buffer.put_str(&self.current_command);
                    }
                }
                DecodedKey::RawKey(KeyCode::Return) => {
                    let is_task_running = self.is_task_running.load(Ordering::SeqCst);
                    if !is_task_running {
                        self.text_buffer.put_char('\n');
                        // Here you can process the command stored in self.current_command
                        self.handle_command();
                        self.command_history.push(self.current_command.clone());
                        self.command_index = self.command_history.len();
                        self.current_command.clear();
                        let is_still_running = self.is_task_running.load(Ordering::SeqCst);
                        if !is_still_running {
                            stdout::flush(self);
                            self.text_buffer.put_str("> ");
                        }
                    } else {
                        stdin::write(b"\n");
                    }
                }
                DecodedKey::RawKey(KeyCode::Backspace) => {
                    let is_task_running = self.is_task_running.load(Ordering::SeqCst);
                    if is_task_running {
                        let _ = stdin::pop();
                    } else {
                        if !self.current_command.is_empty() {
                            self.current_command.pop();
                            self.text_buffer.backspace();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl<F: FrameBuffer, P: PsfFont> core::fmt::Write for Console<'_, F, P> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Console::write_str(self, s);
        Ok(())
    }
}

pub extern "C" fn console_task() {
    x86_64::instructions::interrupts::enable();
    let mut console = unsafe {
        Console::new(
            GLOBAL_CONTEXT.framebuffer.as_mut().unwrap().deref_mut(),
            &*FONT_DEFAULT,
        )
    };

    loop {
        while let Some(event) = x86_64::instructions::interrupts::without_interrupts(|| unsafe {
            GLOBAL_CONTEXT.pop_key()
        }) {
            console.handle_key_event(event);
        }

        let is_running = console.is_task_running.load(Ordering::SeqCst);
        if !is_running && console.was_task_running {
            stdout::flush(&mut console);
            console.text_buffer.put_str("> ");
        }
        console.was_task_running = is_running;

        stdout::flush(&mut console);

        x86_64::instructions::hlt();
        yield_now();
    }
}
