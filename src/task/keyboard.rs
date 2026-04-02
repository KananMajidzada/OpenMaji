use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{DecodedKey, HandleControl, Keyboard, ScancodeSet1, layouts, KeyCode};
use alloc::string::String;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            WAKER.wake();
        } else {
            WAKER.wake();
        }
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE.try_get().expect("scancode queue not initialized");
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }
        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

// --- NEW: Command Processing Logic ---
fn interpret_command(command: &str) {
    let cmd = command.trim();
    if cmd.is_empty() { return; }

    match cmd {
        "help" => {
            println!("\nMAJI OS COMMANDS:");
            println!("  help     - Show this menu");
            println!("  list     - List all MajiFS resources");
            println!("  clear    - Clear the terminal");
            println!("  version  - Show kernel version");
        },
        "list" => {
            println!("\nScanning MajiFS...");
            crate::fs::print_all_resources();
        },
        "clear" => {
            crate::vga_buffer::clear_screen();
        },
        "version" => {
            println!("\nMaji OS v0.1.0-alpha (Resource-Based)");
        },
        _ => {
            println!("\nUnknown resource request: '{}'", cmd);
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );
    
    // Buffer to hold current line of text
    let mut command_buffer = String::new();

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        match character {
                            '\n' => {
                                interpret_command(&command_buffer);
                                command_buffer.clear();
                                // New prompt is handled by vga_buffer's new_line
                                print!("\n"); 
                            },
                            '\u{0008}' => { // Backspace
                                if !command_buffer.is_empty() {
                                    command_buffer.pop();
                                    crate::vga_buffer::backspace();
                                }
                            },
                            c => {
                                command_buffer.push(c);
                                print!("{}", c);
                            }
                        }
                    },
                    DecodedKey::RawKey(raw_key) => {
                        if raw_key == KeyCode::Backspace {
                            if !command_buffer.is_empty() {
                                command_buffer.pop();
                                crate::vga_buffer::backspace();
                            }
                        }
                    }
                }
            }
        }
    }
}