use alloc::collections::VecDeque;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use super::{Process, ProcessState, Context};

// ── Scheduler ─────────────────────────────────────────────────────────────────

pub struct Scheduler {
    processes:   Vec<Process>,
    current:     Option<usize>,  
    ready_queue: VecDeque<usize>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            processes:   Vec::new(),
            current:     None,
            ready_queue: VecDeque::new(),
        }
    }


    pub fn spawn(&mut self, process: Process) {
        let index = self.processes.len();
        self.processes.push(process);
        self.ready_queue.push_back(index);
        crate::println!("[scheduler] spawned '{}' pid={}",
            self.processes[index].name,
            self.processes[index].pid.as_u64()
        );
    }

  
    pub fn schedule(&mut self) -> Option<(*mut Context, *const Context)> {
        
        if let Some(curr_idx) = self.current {
            let proc = &mut self.processes[curr_idx];
            if proc.state == ProcessState::Running {
                proc.state = ProcessState::Ready;
                self.ready_queue.push_back(curr_idx);
            }
        }

       
        let next_idx = loop {
            match self.ready_queue.pop_front() {
                None => return None, 
                Some(idx) => {
                    if self.processes[idx].is_alive() {
                        break idx;
                    }
                    
                }
            }
        };

        let old_ctx_ptr = if let Some(curr_idx) = self.current {
            &mut self.processes[curr_idx].context as *mut Context
        } else {
            core::ptr::null_mut()
        };

        self.processes[next_idx].state = ProcessState::Running;
        self.current = Some(next_idx);
        let new_ctx_ptr = &self.processes[next_idx].context as *const Context;

        if old_ctx_ptr.is_null() {
            None // first ever schedule, nothing to save
        } else {
            Some((old_ctx_ptr, new_ctx_ptr))
        }
    }

   
    pub fn kill(&mut self, pid: u64) -> Result<(), &'static str> {
        match self.processes.iter_mut().find(|p| p.pid.as_u64() == pid) {
            None => Err("process not found"),
            Some(p) => {
                p.state = ProcessState::Dead;
                crate::println!("[scheduler] killed pid={}", pid);
                Ok(())
            }
        }
    }

    
    pub fn list(&self) -> Vec<(u64, &str, ProcessState)> {
        self.processes
            .iter()
            .filter(|p| p.is_alive())
            .map(|p| (p.pid.as_u64(), p.name.as_str(), p.state))
            .collect()
    }

    pub fn process_count(&self) -> usize {
        self.processes.iter().filter(|p| p.is_alive()).count()
    }
}



lazy_static! {
    pub static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}


#[unsafe(naked)]
unsafe extern "C" fn switch_to(old_ctx: *mut Context, new_ctx: *const Context) {
    core::arch::naked_asm!(
 
        "mov [rdi + 0x00], rsp",
        "mov [rdi + 0x08], r15",
        "mov [rdi + 0x10], r14",
        "mov [rdi + 0x18], r13",
        "mov [rdi + 0x20], r12",
        "mov [rdi + 0x28], rbx",
        "mov [rdi + 0x30], rbp",
 
        "mov rax, [rsp]",
        "mov [rdi + 0x38], rax",

       
        "mov rsp, [rsi + 0x00]",
        "mov r15, [rsi + 0x08]",
        "mov r14, [rsi + 0x10]",
        "mov r13, [rsi + 0x18]",
        "mov r12, [rsi + 0x20]",
        "mov rbx, [rsi + 0x28]",
        "mov rbp, [rsi + 0x30]",
        "mov rax, [rsi + 0x38]",
       
        "mov [rsp], rax",
        "ret",
    );
}


pub fn try_switch() {
    let switch = SCHEDULER.lock().schedule();
    if let Some((old_ctx, new_ctx)) = switch {
        unsafe { switch_to(old_ctx, new_ctx); }
    }
}