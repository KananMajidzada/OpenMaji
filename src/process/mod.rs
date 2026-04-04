pub mod scheduler;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(u64);

impl Pid {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Pid(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,  
    Running, 
    Blocked,  
    Dead,    
}



#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct Context {
    pub rsp: u64, 
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64, 
    pub rip: u64,  
}


pub struct Process {
    pub pid:     Pid,
    pub name:    String,
    pub state:   ProcessState,
    pub context: Context,
    pub stack:   alloc::vec::Vec<u8>,  
}

impl Process {

    pub fn new(name: &str, entry_point: fn() -> !) -> Self {
        const STACK_SIZE: usize = 4096 * 4; 
        let mut stack = alloc::vec![0u8; STACK_SIZE];

        
        let stack_top = stack.as_mut_ptr() as usize + STACK_SIZE;

      
        let stack_top = stack_top & !0xF;

        let context = Context {
            rsp: stack_top as u64,
            rip: entry_point as u64,
            ..Default::default()
        };

        Process {
            pid: Pid::new(),
            name: alloc::string::String::from(name),
            state: ProcessState::Ready,
            context,
            stack,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.state != ProcessState::Dead
    }
}