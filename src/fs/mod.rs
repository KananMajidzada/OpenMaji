pub mod disk;
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use spin::Mutex;
use lazy_static::lazy_static;

const MAX_INODES: usize = 256;
const MAX_FILE_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub struct Inode {
    pub id:       usize,
    pub kind:     InodeKind,
    pub name:     String,
    pub data:     Vec<u8>,
    pub children: BTreeMap<String, usize>, 
    pub parent:   usize,                  
}
pub struct OpenMajiFs {
    pub inodes: Vec<Option<Inode>>,  
    pub cwd:    usize,               
}


impl OpenMajiFs {
    pub fn new() -> Self {
        let mut inodes: Vec<Option<Inode>> = (0..MAX_INODES).map(|_| None).collect();

        
        inodes[0] = Some(Inode {
            id:       0,
            kind:     InodeKind::Directory,
            name:     String::from("/"),
            data:     Vec::new(),
            children: BTreeMap::new(),
            parent:   0, 
        });

        OpenMajiFs { inodes, cwd: 0 }
    }

   

    fn alloc_inode(&mut self, kind: InodeKind, name: &str, parent: usize) -> Result<usize, &'static str> {
        let id = self.inodes.iter().position(|slot| slot.is_none())
            .ok_or("FS Error: No free inodes available")?;
        
        self.inodes[id] = Some(Inode {
            id,
            kind,
            name: String::from(name),
            data: Vec::new(),
            children: BTreeMap::new(),
            parent,
        });
        Ok(id)
    }

    fn get_inode(&self, id: usize) -> &Inode {
        self.inodes[id].as_ref().expect("FS Error: Inode corruption")
    }

    fn get_inode_mut(&mut self, id: usize) -> &mut Inode {
        self.inodes[id].as_mut().expect("FS Error: Inode corruption")
    }

    // --- Public API ---

    pub fn cwd_name(&self) -> &str {
        &self.get_inode(self.cwd).name
    }

    pub fn cwd_path(&self) -> String {
        if self.cwd == 0 { return String::from("/"); }
        let mut components = Vec::new();
        let mut current = self.cwd;
        while current != 0 {
            let node = self.get_inode(current);
            components.push(node.name.clone());
            current = node.parent;
        }
        let mut path = String::new();
        for comp in components.iter().rev() {
            path.push('/');
            path.push_str(comp);
        }
        path
    }

    pub fn list(&self) -> Vec<(String, bool)> {
        self.get_inode(self.cwd)
            .children
            .iter()
            .map(|(name, &id)| {
                let is_dir = matches!(self.get_inode(id).kind, InodeKind::Directory);
                (name.clone(), is_dir)
            })
            .collect()
    }

    pub fn mkdir(&mut self, name: &str) -> Result<(), &'static str> {
        if name.is_empty() { return Err("Name cannot be empty"); }
        if self.get_inode(self.cwd).children.contains_key(name) { return Err("Entry already exists"); }
        let id = self.alloc_inode(InodeKind::Directory, name, self.cwd)?;
        self.get_inode_mut(self.cwd).children.insert(name.to_string(), id);
        Ok(())
    }

    pub fn create(&mut self, name: &str) -> Result<(), &'static str> {
        if name.is_empty() { return Err("Name cannot be empty"); }
        if self.get_inode(self.cwd).children.contains_key(name) { return Err("Entry already exists"); }
        let id = self.alloc_inode(InodeKind::File, name, self.cwd)?;
        self.get_inode_mut(self.cwd).children.insert(name.to_string(), id);
        Ok(())
    }

    pub fn write(&mut self, name: &str, content: &[u8]) -> Result<(), &'static str> {
        if content.len() > MAX_FILE_SIZE { return Err("File size exceeds limit"); }
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("File not found")?;
        let node = self.get_inode_mut(id);
        if node.kind == InodeKind::Directory { return Err("Cannot write to a directory"); }
        node.data = content.to_vec();
        Ok(())
    }

    // Changed back to returning &[u8] to fix the shell.rs mismatched types errors
    pub fn read(&self, name: &str) -> Result<&[u8], &'static str> {
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("File not found")?;
        let node = self.get_inode(id);
        if node.kind == InodeKind::Directory { return Err("Is a directory"); }
        Ok(&node.data)
    }

    pub fn append(&mut self, name: &str, content: &[u8]) -> Result<(), &'static str> {
        let mut data = self.read(name)?.to_vec();
        if data.len() + content.len() > MAX_FILE_SIZE { return Err("File too large"); }
        data.extend_from_slice(content);
        self.write(name, &data)
    }

    pub fn file_size(&self, name: &str) -> Result<usize, &'static str> {
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("Not found")?;
        Ok(self.get_inode(id).data.len())
    }

    pub fn remove(&mut self, name: &str) -> Result<(), &'static str> {
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("Not found")?;
        if self.get_inode(id).kind == InodeKind::Directory && !self.get_inode(id).children.is_empty() {
            return Err("Directory not empty");
        }
        self.get_inode_mut(self.cwd).children.remove(name);
        self.inodes[id] = None;
        Ok(())
    }

    pub fn remove_recursive(&mut self, name: &str) -> Result<(), &'static str> {
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("Not found")?;
        self.recursive_delete_helper(id);
        self.get_inode_mut(self.cwd).children.remove(name);
        Ok(())
    }

    fn recursive_delete_helper(&mut self, id: usize) {
        let child_ids: Vec<usize> = self.get_inode(id).children.values().cloned().collect();
        for cid in child_ids {
            self.recursive_delete_helper(cid);
        }
        self.inodes[id] = None;
    }

    pub fn cd(&mut self, name: &str) -> Result<(), &'static str> {
        if name == ".." {
            self.cwd = self.get_inode(self.cwd).parent;
            return Ok(());
        }
        if name == "/" {
            self.cwd = 0;
            return Ok(());
        }
        let id = *self.get_inode(self.cwd).children.get(name).ok_or("Not found")?;
        if self.get_inode(id).kind != InodeKind::Directory { return Err("Not a directory"); }
        self.cwd = id;
        Ok(())
    }

    pub fn copy(&mut self, src: &str, dst: &str) -> Result<(), &'static str> {
        let data = self.read(src)?.to_vec();
        self.create(dst)?;
        self.write(dst, &data)
    }

    pub fn rename(&mut self, src: &str, dst: &str) -> Result<(), &'static str> {
        self.copy(src, dst)?;
        self.remove(src)
    }

    // Added find method for the shell
    pub fn find(&self, name: &str) -> Vec<String> {
        self.get_inode(self.cwd)
            .children
            .keys()
            .filter(|k| k.contains(name))
            .cloned()
            .collect()
    }

    pub fn stats(&self) -> (usize, usize, usize, usize, usize) {
        let used = self.inodes.iter().filter(|i| i.is_some()).count();
        let mut files = 0;
        let mut dirs = 0;
        let mut total_size = 0;
        for node in self.inodes.iter().flatten() {
            match node.kind {
                InodeKind::File => { files += 1; total_size += node.data.len(); }
                InodeKind::Directory => dirs += 1,
            }
        }
        (used, MAX_INODES, files, dirs, total_size)
    }
}

lazy_static! {
    pub static ref OPENMAJIFS: Mutex<OpenMajiFs> = Mutex::new(OpenMajiFs::new());
}