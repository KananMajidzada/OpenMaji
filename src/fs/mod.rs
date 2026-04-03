use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use spin::Mutex;
use lazy_static::lazy_static;



const MAX_INODES: usize = 256;
const MAX_FILE_SIZE: usize = 4096;



#[derive(Debug, Clone)]
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
    pub children: BTreeMap<String, usize>, // name → inode id (dirs only)
    pub parent:   usize,                   // parent inode id
}



pub struct MajiFs {
    inodes:  Vec<Option<Inode>>,
    next_id: usize,
    cwd:     usize,
}

impl MajiFs {
    pub fn new() -> Self {
        let mut inodes: Vec<Option<Inode>> = (0..MAX_INODES).map(|_| None).collect();

        
        inodes[0] = Some(Inode {
            id:       0,
            kind:     InodeKind::Directory,
            name:     String::from("/"),
            data:     Vec::new(),
            children: BTreeMap::new(),
            parent:   0, // root's parent is itself
        });

        MajiFs { inodes, next_id: 1, cwd: 0 }
    }

   

    fn alloc_inode(&mut self, kind: InodeKind, name: &str, parent: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.inodes[id] = Some(Inode {
            id,
            kind,
            name: String::from(name),
            data: Vec::new(),
            children: BTreeMap::new(),
            parent,
        });
        id
    }

    fn cwd_ref(&self) -> &Inode {
        self.inodes[self.cwd].as_ref().unwrap()
    }

    fn cwd_mut(&mut self) -> &mut Inode {
        self.inodes[self.cwd].as_mut().unwrap()
    }

    


    pub fn cwd_name(&self) -> &str {
        &self.cwd_ref().name
    }

    
    pub fn cwd_path(&self) -> String {
        let mut path = Vec::new();
        let mut current = self.cwd;
        loop {
            let inode = self.inodes[current].as_ref().unwrap();
            if current == 0 {
                break;
            }
            path.push(inode.name.clone());
            current = inode.parent;
        }
        if path.is_empty() {
            return String::from("/");
        }
        let mut result = String::new();
        for part in path.iter().rev() {
            result.push('/');
            result.push_str(part);
        }
        result
    }

   
    pub fn list(&self) -> Vec<(String, bool)> {
        self.cwd_ref()
            .children
            .iter()
            .map(|(name, &id)| {
                let is_dir = matches!(
                    self.inodes[id].as_ref().unwrap().kind,
                    InodeKind::Directory
                );
                (name.clone(), is_dir)
            })
            .collect()
    }

   
    pub fn mkdir(&mut self, name: &str) -> Result<(), &'static str> {
        if name.is_empty() { return Err("name cannot be empty"); }
        if self.cwd_ref().children.contains_key(name) {
            return Err("already exists");
        }
        let parent = self.cwd;
        let id = self.alloc_inode(InodeKind::Directory, name, parent);
        self.cwd_mut().children.insert(name.to_string(), id);
        Ok(())
    }

   
    pub fn create(&mut self, name: &str) -> Result<(), &'static str> {
        if name.is_empty() { return Err("name cannot be empty"); }
        if self.cwd_ref().children.contains_key(name) {
            return Err("already exists");
        }
        let parent = self.cwd;
        let id = self.alloc_inode(InodeKind::File, name, parent);
        self.cwd_mut().children.insert(name.to_string(), id);
        Ok(())
    }

  
    pub fn write(&mut self, name: &str, content: &[u8]) -> Result<(), &'static str> {
        if content.len() > MAX_FILE_SIZE { return Err("file too large"); }
        let id = *self.cwd_ref().children.get(name).ok_or("not found")?;
        match self.inodes[id].as_ref().unwrap().kind {
            InodeKind::Directory => return Err("is a directory"),
            InodeKind::File => {}
        }
        self.inodes[id].as_mut().unwrap().data = content.to_vec();
        Ok(())
    }

    pub fn read(&self, name: &str) -> Result<&[u8], &'static str> {
        let id = *self.cwd_ref().children.get(name).ok_or("not found")?;
        match self.inodes[id].as_ref().unwrap().kind {
            InodeKind::Directory => Err("is a directory"),
            InodeKind::File => Ok(&self.inodes[id].as_ref().unwrap().data),
        }
    }

   
    pub fn remove(&mut self, name: &str) -> Result<(), &'static str> {
        let id = *self.cwd_ref().children.get(name).ok_or("not found")?;
        let inode = self.inodes[id].as_ref().unwrap();
        if let InodeKind::Directory = inode.kind {
            if !inode.children.is_empty() {
                return Err("directory not empty");
            }
        }
        self.cwd_mut().children.remove(name);
        self.inodes[id] = None;
        Ok(())
    }

   
    pub fn cd(&mut self, name: &str) -> Result<(), &'static str> {
        if name == ".." {
            let parent = self.cwd_ref().parent;
            self.cwd = parent;
            return Ok(());
        }
        if name == "/" {
            self.cwd = 0;
            return Ok(());
        }
        let id = *self.cwd_ref().children.get(name).ok_or("not found")?;
        match self.inodes[id].as_ref().unwrap().kind {
            InodeKind::Directory => { self.cwd = id; Ok(()) }
            InodeKind::File      => Err("not a directory"),
        }
    }

    
    pub fn file_size(&self, name: &str) -> Result<usize, &'static str> {
        let id = *self.cwd_ref().children.get(name).ok_or("not found")?;
        Ok(self.inodes[id].as_ref().unwrap().data.len())
    }
}



lazy_static! {
    pub static ref MAJIFS: Mutex<MajiFs> = Mutex::new(MajiFs::new());
}