use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0, Blue = 1, Green = 2, Cyan = 3, Red = 4, Magenta = 5, Brown = 6, LightGray = 7,
    DarkGray = 8, LightBlue = 9, LightGreen = 10, LightCyan = 11, LightRed = 12, Pink = 13, Yellow = 14, White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
        // no prompt here — shell.prompt() handles it
    }

    pub fn draw_status_bar(&mut self) {
        let status_color = ColorCode::new(Color::Black, Color::LightGray);
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[0][col].write(ScreenChar {
                ascii_character: b' ',
                color_code: status_color,
            });
        }
        let title = " OpenMaji ";
        let start_x = (BUFFER_WIDTH / 2) - (title.len() / 2);
        for (i, byte) in title.bytes().enumerate() {
            self.buffer.chars[0][start_x + i].write(ScreenChar {
                ascii_character: byte,
                color_code: status_color,
            });
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.draw_status_bar();
        self.column_position = 0;
    }

    pub fn backspace(&mut self) {
        if self.column_position > 0 {
            self.column_position -= 1;
            let row = BUFFER_HEIGHT - 1;
            self.buffer.chars[row][self.column_position].write(ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            });
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg);
    }

    pub fn reset_color(&mut self) {
        self.color_code = ColorCode::new(Color::White, Color::Black);
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// ── Public free functions ─────────────────────────────────────────────────────

pub fn clear_screen() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().clear_screen();
    });
}

pub fn backspace() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().backspace();
    });
}

pub fn set_color(fg: Color, bg: Color) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().set_color(fg, bg);
    });
}

pub fn reset_color() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().reset_color();
    });
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}