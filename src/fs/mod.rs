pub mod mem;

use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use self::mem::RamScheme;

pub type FsResult<T> = Result<T, &'static str>;

pub trait MajiScheme {
    fn read(&self, path: &str) -> FsResult<Vec<u8>>;
    // Fixed: Removed the duplicate/incorrect 'path: &mut self'
    fn write(&mut self, path: &str, data: &[u8]) -> FsResult<()>;
    fn list(&self) -> Vec<String>;
}

lazy_static! {
    pub static ref MEM_FS: Mutex<RamScheme> = Mutex::new(RamScheme::new());
}

pub fn store_resource(name: &str, data: &str) {
    let mut fs = MEM_FS.lock();
    let _ = fs.write(name, data.as_bytes());
}

pub fn print_all_resources() {
    let fs = MEM_FS.lock();
    let files = fs.list();
    if files.is_empty() {
        crate::println!("MajiFS is empty.");
    } else {
        for name in files {
            crate::println!("  mem:{}", name);
        }
    }
}