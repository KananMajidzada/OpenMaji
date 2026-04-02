use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::{MajiScheme, FsResult};

pub struct RamScheme {
    // Stores resources as key-value pairs in memory
    files: BTreeMap<String, Vec<u8>>,
}

impl RamScheme {
    pub fn new() -> Self {
        Self { files: BTreeMap::new() }
    }
}

impl MajiScheme for RamScheme {
    fn read(&self, path: &str) -> FsResult<Vec<u8>> {
        self.files.get(path).cloned().ok_or("Resource not found")
    }

    fn write(&mut self, path: &str, data: &[u8]) -> FsResult<()> {
        self.files.insert(path.to_string(), data.to_vec());
        Ok(())
    }

    fn list(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }
}