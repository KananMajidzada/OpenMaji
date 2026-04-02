#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(maji::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use maji::task::{Task, executor::Executor, keyboard};
use maji::println;
use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use maji::allocator;
    use maji::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    // 1. Hardware abstraction layer initialization
    maji::init();

    // 2. Memory & Heap initialization
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { 
        BootInfoFrameAllocator::init(&boot_info.memory_map) 
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    // 3. MajiFS Resource Setup
    maji::vga_buffer::clear_screen();
    
    // Register initial resources
    maji::fs::store_resource("sys/kernel", "Maji Kernel v0.1.0");
    maji::fs::store_resource("sys/fs", "MajiFS (Resource-Based)");
    maji::fs::store_resource("dev/vga", "0xb8000");

    println!("Maji Kernel started!");
    println!("Listing MajiFS resources:");
    maji::fs::print_all_resources();

    #[cfg(test)]
    test_main();

    // 4. Start Multitasking
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    maji::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    maji::test_panic_handler(info)
}