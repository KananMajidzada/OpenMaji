use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::fs::MAJIFS;
use crate::{print, println};
use crate::vga_buffer::Color;

// ── Global Shell instance ─────────────────────────────────────────────────────

lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());
}

// ── Shell ─────────────────────────────────────────────────────────────────────

pub struct Shell {
    input: String,
}

impl Shell {
    pub fn new() -> Self {
        Shell { input: String::new() }
    }

    pub fn prompt(&self) {
        let fs = MAJIFS.lock();
        let path = fs.cwd_path();
        drop(fs);
        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
        print!("maji");
        crate::vga_buffer::set_color(Color::White, Color::Black);
        print!(":");
        crate::vga_buffer::set_color(Color::LightBlue, Color::Black);
        print!("{}", path);
        crate::vga_buffer::set_color(Color::White, Color::Black);
        print!("> ");
        crate::vga_buffer::reset_color();
    }

    pub fn handle_char(&mut self, c: char) {
        match c {
            '\n' => {
                println!();
                let cmd = self.input.clone();
                self.input.clear();
                self.run(&cmd);
                self.prompt();
            }
            '\u{0008}' => {
                if !self.input.is_empty() {
                    self.input.pop();
                    crate::vga_buffer::backspace();
                }
            }
            c => {
                self.input.push(c);
                print!("{}", c);
            }
        }
    }

    // ── Command dispatcher ────────────────────────────────────────────────────

    fn run(&self, line: &str) {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() { return; }

        match parts[0] {
            "help"    => self.cmd_help(),
            "clear"   => self.cmd_clear(),
            "echo"    => self.cmd_echo(&parts[1..]),
            "ls"      => self.cmd_ls(),
            "mkdir"   => self.cmd_mkdir(parts.get(1).copied()),
            "touch"   => self.cmd_touch(parts.get(1).copied()),
            "rm"      => self.cmd_rm(parts.get(1).copied()),
            "cat"     => self.cmd_cat(parts.get(1).copied()),
            "write"   => self.cmd_write(parts.get(1).copied(), &parts[2..]),
            "cd"      => self.cmd_cd(parts.get(1).copied()),
            "pwd"     => self.cmd_pwd(),
            "version" => self.cmd_version(),
            unknown   => {
                crate::vga_buffer::set_color(Color::LightRed, Color::Black);
                println!("unknown command: '{}'", unknown);
                crate::vga_buffer::reset_color();
                println!("type 'help' for a list of commands");
            }
        }
    }

    // ── Commands ──────────────────────────────────────────────────────────────

    fn cmd_help(&self) {
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("Command interface");
        crate::vga_buffer::reset_color();
        println!();
        println!("  ls               list current directory");
        println!("  pwd              print working directory");
        println!("  cd <dir>         change directory (.. to go up)");
        println!("  mkdir <name>     create a directory");
        println!("  touch <name>     create an empty file");
        println!("  rm <name>        remove a file or empty directory");
        println!("  cat <name>       print file contents");
        println!("  write <n> <txt>  write text to file");
        println!("  echo <text>      print text");
        println!("  clear            clear the screen");
        println!("  version          show kernel version");
    }

    fn cmd_clear(&self) {
        crate::vga_buffer::clear_screen();
    }

    fn cmd_echo(&self, args: &[&str]) {
        println!("{}", args.join(" "));
    }

    fn cmd_pwd(&self) {
        let fs = MAJIFS.lock();
        println!("{}", fs.cwd_path());
    }

    fn cmd_ls(&self) {
        let fs = MAJIFS.lock();
        let mut entries = fs.list();
        drop(fs);

        if entries.is_empty() {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            println!("(empty)");
            crate::vga_buffer::reset_color();
            return;
        }

        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        for (name, is_dir) in entries {
            if is_dir {
                crate::vga_buffer::set_color(Color::LightBlue, Color::Black);
                println!("  {}/", name);
                crate::vga_buffer::reset_color();
            } else {
                let fs = MAJIFS.lock();
                let size = fs.file_size(&name).unwrap_or(0);
                drop(fs);
                crate::vga_buffer::reset_color();
                println!("  {}  ({} B)", name, size);
            }
        }
    }

    fn cmd_mkdir(&self, name: Option<&str>) {
        match name {
            None => println!("usage: mkdir <name>"),
            Some(n) => match MAJIFS.lock().mkdir(n) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("created {}/", n);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("mkdir", e),
            }
        }
    }

    fn cmd_touch(&self, name: Option<&str>) {
        match name {
            None => println!("usage: touch <name>"),
            Some(n) => match MAJIFS.lock().create(n) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("created {}", n);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("touch", e),
            }
        }
    }

    fn cmd_rm(&self, name: Option<&str>) {
        match name {
            None => println!("usage: rm <name>"),
            Some(n) => match MAJIFS.lock().remove(n) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("removed {}", n);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("rm", e),
            }
        }
    }

    fn cmd_cat(&self, name: Option<&str>) {
        match name {
            None => println!("usage: cat <name>"),
            Some(n) => {
                let fs = MAJIFS.lock();
                match fs.read(n) {
                    Ok(data) => {
                        let text = core::str::from_utf8(data)
                            .unwrap_or("(binary file)");
                        println!("{}", text);
                    }
                    Err(e) => {
                        drop(fs);
                        self.print_err("cat", e);
                    }
                }
            }
        }
    }

    fn cmd_write(&self, name: Option<&str>, args: &[&str]) {
        match name {
            None => println!("usage: write <name> <content>"),
            Some(n) => {
                if args.is_empty() {
                    println!("usage: write <name> <content>");
                    return;
                }
                let content = args.join(" ");
                match MAJIFS.lock().write(n, content.as_bytes()) {
                    Ok(_) => {
                        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                        println!("wrote {} bytes to {}", content.len(), n);
                        crate::vga_buffer::reset_color();
                    }
                    Err(e) => self.print_err("write", e),
                }
            }
        }
    }

    fn cmd_cd(&self, name: Option<&str>) {
        match name {
            None => println!("usage: cd <name>"),
            Some(n) => match MAJIFS.lock().cd(n) {
                Ok(_)  => {}
                Err(e) => self.print_err("cd", e),
            }
        }
    }

    fn cmd_version(&self) {
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("MajiOS v0.1.0-alpha");
        crate::vga_buffer::reset_color();
        println!("  kernel:  Rust (no_std)");
        println!("  fs:      MajiFS (inode-based, in-memory)");
        println!("  shell:   MajiShell");
    }

    fn print_err(&self, cmd: &str, e: &str) {
        crate::vga_buffer::set_color(Color::LightRed, Color::Black);
        println!("{}: {}", cmd, e);
        crate::vga_buffer::reset_color();
    }
}