use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::fs::OPENMAJIFS;
use crate::{print, println};
use crate::vga_buffer::Color;

lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());
}

pub struct Shell {
    input:   String,
    history: VecDeque<String>,
    aliases: Vec<(String, String)>,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            input:   String::new(),
            history: VecDeque::new(),
            aliases: Vec::new(),
        }
    }

    pub fn prompt(&self) {
        let fs = OPENMAJIFS.lock();
        let path = fs.cwd_path();
        drop(fs);
        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
        print!("openmaji");
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
                if !cmd.trim().is_empty() {
                    if self.history.len() >= 50 {
                        self.history.pop_front();
                    }
                    self.history.push_back(cmd.clone());
                }
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

    fn run(&mut self, line: &str) {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() { return; }

        let resolved = self.resolve_alias(parts[0]);
        let resolved_parts: Vec<&str> = resolved.split_whitespace().collect();
        let cmd = resolved_parts[0];
        let all_parts: Vec<&str> = resolved_parts.iter()
            .chain(parts[1..].iter())
            .copied()
            .collect();

        match cmd {
            "help"    => self.cmd_help(),
            "clear"   => self.cmd_clear(),
            "echo"    => self.cmd_echo(&all_parts[1..]),
            "ls"      => self.cmd_ls(&all_parts[1..]),
            "mkdir"   => self.cmd_mkdir(all_parts.get(1).copied()),
            "touch"   => self.cmd_touch(all_parts.get(1).copied()),
            "rm"      => self.cmd_rm(&all_parts[1..]),
            "cp"      => self.cmd_cp(all_parts.get(1).copied(), all_parts.get(2).copied()),
            "mv"      => self.cmd_mv(all_parts.get(1).copied(), all_parts.get(2).copied()),
            "cat"     => self.cmd_cat(all_parts.get(1).copied()),
            "write"   => self.cmd_write(all_parts.get(1).copied(), &all_parts[2..]),
            "append"  => self.cmd_append(all_parts.get(1).copied(), &all_parts[2..]),
            "cd"      => self.cmd_cd(all_parts.get(1).copied()),
            "pwd"     => self.cmd_pwd(),
            "find"    => self.cmd_find(all_parts.get(1).copied()),
            "grep"    => self.cmd_grep(all_parts.get(1).copied(), all_parts.get(2).copied()),
            "head"    => self.cmd_head(all_parts.get(1).copied(), all_parts.get(2).copied()),
            "tail"    => self.cmd_tail(all_parts.get(1).copied(), all_parts.get(2).copied()),
            "history" => self.cmd_history(),
            "alias"   => self.cmd_alias(all_parts.get(1).copied()),
            "unalias" => self.cmd_unalias(all_parts.get(1).copied()),
            "whoami"  => self.cmd_whoami(),
            "uname"   => self.cmd_uname(),
            "uptime"  => self.cmd_uptime(),
            "df"      => self.cmd_df(),
            "free"    => self.cmd_free(),
            "ps"      => self.cmd_ps(),
            "man"     => self.cmd_man(all_parts.get(1).copied()),
            "fs"      => self.cmd_openmajifs(),
            "save"    => self.cmd_save(),
            "load"    => self.cmd_load(),
            "format"  => self.cmd_format(),
            "about"   => self.cmd_about(),
            "exit"    => self.cmd_exit(),
            "version" => self.cmd_version(),
            unknown   => {
                crate::vga_buffer::set_color(Color::LightRed, Color::Black);
                println!("unknown command: '{}'", unknown);
                crate::vga_buffer::reset_color();
                println!("type 'help' for a list of commands");
            }
        }
    }

    fn resolve_alias(&self, cmd: &str) -> String {
        for (name, value) in &self.aliases {
            if name == cmd {
                return value.clone();
            }
        }
        String::from(cmd)
    }

    fn cmd_help(&self) {
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("Shell - OpenMaji command interface");
        crate::vga_buffer::reset_color();
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("Navigation:");
        crate::vga_buffer::reset_color();
        println!("  ls [-la]         list directory (use -la for hidden files)");
        println!("  pwd              print working directory");
        println!("  cd <dir>         change directory (.. to go up, / for root)");
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("File operations:");
        crate::vga_buffer::reset_color();
        println!("  touch <name>     create empty file");
        println!("  mkdir <name>     create directory");
        println!("  cp <src> <dst>   copy file");
        println!("  mv <src> <dst>   move or rename file");
        println!("  rm <name>        remove file");
        println!("  rm -rf <name>    force remove directory and contents");
        println!("  find <name>      search for file by name");
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("Viewing & editing:");
        crate::vga_buffer::reset_color();
        println!("  cat <name>       print file contents");
        println!("  head <name> [n]  print first n lines (default 5)");
        println!("  tail <name> [n]  print last n lines (default 5)");
        println!("  grep <pat> <f>   search for pattern in file");
        println!("  write <n> <txt>  write text to file");
        println!("  append <n> <txt> append text to file");
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("Disk:");
        crate::vga_buffer::reset_color();
        println!("  format           format disk for OpenMajiFS");
        println!("  save             save filesystem to disk");
        println!("  load             load filesystem from disk");
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("System:");
        crate::vga_buffer::reset_color();
        println!("  whoami           show current user");
        println!("  uname            show system info");
        println!("  uptime           show system uptime");
        println!("  df               show filesystem usage");
        println!("  free             show memory info");
        println!("  ps               show running tasks");
        println!("  fs               show OpenMajiFS stats");
        println!();
        crate::vga_buffer::set_color(Color::Yellow, Color::Black);
        println!("Misc:");
        crate::vga_buffer::reset_color();
        println!("  echo <text>      print text");
        println!("  history          show command history");
        println!("  alias <n>=<cmd>  create command alias");
        println!("  unalias <name>   remove alias");
        println!("  man <cmd>        show command manual");
        println!("  about            about OpenMajiOS");
        println!("  version          show version");
        println!("  clear            clear screen");
        println!("  exit             halt the system");
    }

    fn cmd_clear(&self) {
        crate::vga_buffer::clear_screen();
    }

    fn cmd_echo(&self, args: &[&str]) {
        println!("{}", args.join(" "));
    }

    fn cmd_pwd(&self) {
        let fs = OPENMAJIFS.lock();
        println!("{}", fs.cwd_path());
    }

    fn cmd_ls(&self, args: &[&str]) {
        let show_hidden = args.contains(&"-la") || args.contains(&"-a");
        let fs = OPENMAJIFS.lock();
        let mut entries = fs.list();
        drop(fs);

        if entries.is_empty() {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            println!("(empty)");
            crate::vga_buffer::reset_color();
            return;
        }

        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        if show_hidden {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            println!("  ./");
            println!("  ../");
            crate::vga_buffer::reset_color();
        }

        for (name, is_dir) in entries {
            if is_dir {
                crate::vga_buffer::set_color(Color::LightBlue, Color::Black);
                println!("  {}/", name);
                crate::vga_buffer::reset_color();
            } else {
                let fs = OPENMAJIFS.lock();
                let size = fs.file_size(&name).unwrap_or(0);
                drop(fs);
                println!("  {}  ({} B)", name, size);
            }
        }
    }

    fn cmd_mkdir(&self, name: Option<&str>) {
        match name {
            None => println!("usage: mkdir <name>"),
            Some(n) => match OPENMAJIFS.lock().mkdir(n) {
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
            Some(n) => match OPENMAJIFS.lock().create(n) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("created {}", n);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("touch", e),
            }
        }
    }

    fn cmd_rm(&self, args: &[&str]) {
        if args.is_empty() {
            println!("usage: rm <name>  or  rm -rf <name>");
            return;
        }
        let force = args.contains(&"-rf") || args.contains(&"-r");
        let name = args.iter().find(|&&a| !a.starts_with('-'));
        match name {
            None => println!("usage: rm <name>"),
            Some(&n) => {
                if force {
                    match OPENMAJIFS.lock().remove_recursive(n) {
                        Ok(_) => {
                            crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                            println!("removed {}", n);
                            crate::vga_buffer::reset_color();
                        }
                        Err(e) => self.print_err("rm -rf", e),
                    }
                } else {
                    match OPENMAJIFS.lock().remove(n) {
                        Ok(_) => {
                            crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                            println!("removed {}", n);
                            crate::vga_buffer::reset_color();
                        }
                        Err(e) => self.print_err("rm", e),
                    }
                }
            }
        }
    }

    fn cmd_cp(&self, src: Option<&str>, dst: Option<&str>) {
        match (src, dst) {
            (Some(s), Some(d)) => match OPENMAJIFS.lock().copy(s, d) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("copied {} -> {}", s, d);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("cp", e),
            }
            _ => println!("usage: cp <src> <dst>"),
        }
    }

    fn cmd_mv(&self, src: Option<&str>, dst: Option<&str>) {
        match (src, dst) {
            (Some(s), Some(d)) => match OPENMAJIFS.lock().rename(s, d) {
                Ok(_) => {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("moved {} -> {}", s, d);
                    crate::vga_buffer::reset_color();
                }
                Err(e) => self.print_err("mv", e),
            }
            _ => println!("usage: mv <src> <dst>"),
        }
    }

    fn cmd_cat(&self, name: Option<&str>) {
        match name {
            None => println!("usage: cat <name>"),
            Some(n) => {
                let fs = OPENMAJIFS.lock();
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

    fn cmd_head(&self, name: Option<&str>, n: Option<&str>) {
        let lines = n.and_then(|s| s.parse::<usize>().ok()).unwrap_or(5);
        match name {
            None => println!("usage: head <name> [lines]"),
            Some(f) => {
                let fs = OPENMAJIFS.lock();
                match fs.read(f) {
                    Ok(data) => {
                        let text = core::str::from_utf8(data).unwrap_or("(binary)");
                        for line in text.lines().take(lines) {
                            println!("{}", line);
                        }
                    }
                    Err(e) => { drop(fs); self.print_err("head", e); }
                }
            }
        }
    }

    fn cmd_tail(&self, name: Option<&str>, n: Option<&str>) {
        let lines = n.and_then(|s| s.parse::<usize>().ok()).unwrap_or(5);
        match name {
            None => println!("usage: tail <name> [lines]"),
            Some(f) => {
                let fs = OPENMAJIFS.lock();
                match fs.read(f) {
                    Ok(data) => {
                        let text = core::str::from_utf8(data).unwrap_or("(binary)");
                        let all: Vec<&str> = text.lines().collect();
                        let start = if all.len() > lines { all.len() - lines } else { 0 };
                        for line in &all[start..] {
                            println!("{}", line);
                        }
                    }
                    Err(e) => { drop(fs); self.print_err("tail", e); }
                }
            }
        }
    }

    fn cmd_grep(&self, pattern: Option<&str>, file: Option<&str>) {
        match (pattern, file) {
            (Some(pat), Some(f)) => {
                let fs = OPENMAJIFS.lock();
                match fs.read(f) {
                    Ok(data) => {
                        let text = core::str::from_utf8(data).unwrap_or("");
                        let mut found = false;
                        for line in text.lines() {
                            if line.contains(pat) {
                                crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                                println!("{}", line);
                                crate::vga_buffer::reset_color();
                                found = true;
                            }
                        }
                        if !found {
                            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
                            println!("no matches for '{}'", pat);
                            crate::vga_buffer::reset_color();
                        }
                    }
                    Err(e) => { drop(fs); self.print_err("grep", e); }
                }
            }
            _ => println!("usage: grep <pattern> <file>"),
        }
    }

    fn cmd_find(&self, name: Option<&str>) {
        match name {
            None => println!("usage: find <name>"),
            Some(n) => {
                let fs = OPENMAJIFS.lock();
                let results = fs.find(n);
                drop(fs);
                if results.is_empty() {
                    crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
                    println!("no files found matching '{}'", n);
                    crate::vga_buffer::reset_color();
                } else {
                    for path in results {
                        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                        println!("{}", path);
                        crate::vga_buffer::reset_color();
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
                match OPENMAJIFS.lock().write(n, content.as_bytes()) {
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

    fn cmd_append(&self, name: Option<&str>, args: &[&str]) {
        match name {
            None => println!("usage: append <name> <content>"),
            Some(n) => {
                if args.is_empty() {
                    println!("usage: append <name> <content>");
                    return;
                }
                let addition = args.join(" ");
                match OPENMAJIFS.lock().append(n, addition.as_bytes()) {
                    Ok(_) => {
                        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                        println!("appended {} bytes to {}", addition.len(), n);
                        crate::vga_buffer::reset_color();
                    }
                    Err(e) => self.print_err("append", e),
                }
            }
        }
    }

    fn cmd_cd(&self, name: Option<&str>) {
        match name {
            None => println!("usage: cd <name>"),
            Some(n) => match OPENMAJIFS.lock().cd(n) {
                Ok(_)  => {}
                Err(e) => self.print_err("cd", e),
            }
        }
    }

    fn cmd_history(&self) {
        if self.history.is_empty() {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            println!("(no history)");
            crate::vga_buffer::reset_color();
            return;
        }
        for (i, cmd) in self.history.iter().enumerate() {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            print!("  {:>3}  ", i + 1);
            crate::vga_buffer::reset_color();
            println!("{}", cmd);
        }
    }

    fn cmd_alias(&mut self, arg: Option<&str>) {
        match arg {
            None => {
                if self.aliases.is_empty() {
                    crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
                    println!("(no aliases)");
                    crate::vga_buffer::reset_color();
                } else {
                    for (name, value) in &self.aliases {
                        println!("  {}='{}'", name, value);
                    }
                }
            }
            Some(a) => {
                if let Some(eq) = a.find('=') {
                    let name = &a[..eq];
                    let value = &a[eq + 1..];
                    let name = name.trim_matches('"').trim_matches('\'');
                    let value = value.trim_matches('"').trim_matches('\'');
                    self.aliases.retain(|(n, _)| n != name);
                    self.aliases.push((String::from(name), String::from(value)));
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("alias {}='{}'", name, value);
                    crate::vga_buffer::reset_color();
                } else {
                    println!("usage: alias <name>=<command>");
                }
            }
        }
    }

    fn cmd_unalias(&mut self, name: Option<&str>) {
        match name {
            None => println!("usage: unalias <name>"),
            Some(n) => {
                let before = self.aliases.len();
                self.aliases.retain(|(a, _)| a != n);
                if self.aliases.len() < before {
                    crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                    println!("removed alias '{}'", n);
                    crate::vga_buffer::reset_color();
                } else {
                    self.print_err("unalias", "alias not found");
                }
            }
        }
    }

    fn cmd_whoami(&self) {
        crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
        println!("openmaji");
        crate::vga_buffer::reset_color();
    }

    fn cmd_uname(&self) {
        println!("OpenMaji  openmaji-kernel  v0.1.0-alpha  x86_64");
    }

    fn cmd_uptime(&self) {
        let ticks = crate::interrupts::ticks();
        let seconds = ticks / 100;
        let minutes = seconds / 60;
        let hours   = minutes / 60;
        println!("up {}h {}m {}s  ({} ticks)",
            hours, minutes % 60, seconds % 60, ticks);
    }

    fn cmd_df(&self) {
        let fs = OPENMAJIFS.lock();
        let stats = fs.stats();
        drop(fs);
        println!("OpenMajiFS (in-memory)");
        println!("  inodes used:  {}/{}", stats.0, stats.1);
        println!("  files:        {}", stats.2);
        println!("  directories:  {}", stats.3);
        println!("  data stored:  {} B", stats.4);
    }

    fn cmd_free(&self) {
        use crate::allocator::{HEAP_START, HEAP_SIZE};
        println!("Heap memory:");
        println!("  start:  0x{:x}", HEAP_START);
        println!("  total:  {} KiB", HEAP_SIZE / 1024);
    }

    fn cmd_ps(&self) {
        use crate::process::ProcessState;

        let list: Vec<(u64, String, ProcessState)> = {
            let sched = crate::process::scheduler::SCHEDULER.lock();
            sched.list()
                .into_iter()
                .map(|(pid, name, state)| (pid, String::from(name), state))
                .collect()
        };

        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("PID  STATE    NAME");
        crate::vga_buffer::reset_color();
        println!("  0  running  kernel");

        if list.is_empty() {
            crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
            println!("  (no user processes)");
            crate::vga_buffer::reset_color();
            return;
        }

        for (pid, name, state) in list {
            let state_str = match state {
                ProcessState::Running => "running",
                ProcessState::Ready   => "ready  ",
                ProcessState::Blocked => "blocked",
                ProcessState::Dead    => "dead   ",
            };
            println!("  {}  {}  {}", pid, state_str, name);
        }
    }

    fn cmd_save(&self) {
        let fs = OPENMAJIFS.lock();
        match crate::fs::disk::save_fs(&fs) {
            Ok(_) => {
                crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                println!("filesystem saved to disk");
                crate::vga_buffer::reset_color();
            }
            Err(e) => self.print_err("save", e),
        }
    }

    fn cmd_load(&self) {
        let mut fs = OPENMAJIFS.lock();
        match crate::fs::disk::load_fs(&mut fs) {
            Ok(_) => {
                crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                println!("filesystem loaded from disk");
                crate::vga_buffer::reset_color();
            }
            Err(e) => self.print_err("load", e),
        }
    }

    fn cmd_format(&self) {
        match crate::fs::disk::format_disk() {
            Ok(_) => {
                crate::vga_buffer::set_color(Color::LightGreen, Color::Black);
                println!("disk formatted for OpenMajiFS");
                crate::vga_buffer::reset_color();
            }
            Err(e) => self.print_err("format", e),
        }
    }

    fn cmd_man(&self, cmd: Option<&str>) {
        match cmd {
            None => println!("usage: man <command>"),
            Some(c) => match c {
                "ls"      => { println!("ls [-la]"); println!("  List files in current directory."); println!("  -la  also show hidden entries (. and ..)"); }
                "cd"      => { println!("cd <dir>"); println!("  Change current directory."); println!("  Use .. to go up, / for root."); }
                "cp"      => { println!("cp <src> <dst>"); println!("  Copy a file to a new name."); }
                "mv"      => { println!("mv <src> <dst>"); println!("  Move or rename a file."); }
                "rm"      => { println!("rm <name>"); println!("  Remove a file."); println!("  rm -rf <name>  force remove directory and all contents."); }
                "grep"    => { println!("grep <pattern> <file>"); println!("  Search for pattern in file, print matching lines."); }
                "head"    => { println!("head <file> [n]"); println!("  Print first n lines of file (default 5)."); }
                "tail"    => { println!("tail <file> [n]"); println!("  Print last n lines of file (default 5)."); }
                "append"  => { println!("append <file> <text>"); println!("  Append text to end of file without overwriting."); }
                "alias"   => { println!("alias <name>=<cmd>"); println!("  Create a shortcut for a command."); println!("  Example: alias ll=ls -la"); }
                "find"    => { println!("find <name>"); println!("  Search for a file by name in current directory."); }
                "uptime"  => { println!("uptime"); println!("  Show how long the system has been running."); }
                "df"      => { println!("df"); println!("  Show OpenMajiFS inode and storage usage."); }
                "free"    => { println!("free"); println!("  Show heap memory info."); }
                "history" => { println!("history"); println!("  Show list of previously entered commands."); }
                "save"    => { println!("save"); println!("  Save the current filesystem to disk."); }
                "load"    => { println!("load"); println!("  Load the filesystem from disk."); }
                "format"  => { println!("format"); println!("  Format the disk for use with OpenMajiFS."); println!("  Warning: destroys all data on disk."); }
                _         => { self.print_err("man", "no manual entry for that command"); }
            }
        }
    }

    fn cmd_openmajifs(&self) {
        let fs = OPENMAJIFS.lock();
        let stats = fs.stats();
        drop(fs);
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("OpenMajiFS - OpenMaji Filesystem");
        crate::vga_buffer::reset_color();
        println!("  type:         inode-based, in-memory + disk");
        println!("  inodes used:  {}/{}", stats.0, stats.1);
        println!("  files:        {}", stats.2);
        println!("  directories:  {}", stats.3);
        println!("  bytes stored: {}", stats.4);
        println!("  max filesize: 4096 B");
        println!("  disk:         ATA PIO (use 'save' and 'load')");
    }

    fn cmd_about(&self) {
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("  OpenMaji Operating System");
        crate::vga_buffer::reset_color();
        println!("  A kernel written in Rust from scratch.");
        println!();
        println!("  author:  Kanan Majidzada");
        println!("  version: v0.1.0-alpha-vivien");
        println!("  arch:    x86_64");
        println!("  lang:    Rust (no_std)");
        println!("  fs:      OpenMajiFS");
        println!("  shell:   OpenMajiShell");
        println!();
        crate::vga_buffer::set_color(Color::DarkGray, Color::Black);
        println!("  github.com/KananMajidzada/OpenMaji");
        crate::vga_buffer::reset_color();
    }

    fn cmd_version(&self) {
        crate::vga_buffer::set_color(Color::LightCyan, Color::Black);
        println!("OpenMajiOS v0.1.0-alpha");
        crate::vga_buffer::reset_color();
        println!("  kernel:  Rust (no_std)");
        println!("  fs:      OpenMajiFS (inode-based, in-memory + disk)");
        println!("  shell:   OpenMajiShell");
    }

    fn cmd_exit(&self) {
        crate::vga_buffer::set_color(Color::LightRed, Color::Black);
        println!("Halting OpenMajiOS...");
        crate::vga_buffer::reset_color();
        crate::hlt_loop();
    }

    fn print_err(&self, cmd: &str, e: &str) {
        crate::vga_buffer::set_color(Color::LightRed, Color::Black);
        println!("{}: {}", cmd, e);
        crate::vga_buffer::reset_color();
    }
}