#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(maji::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use maji::allocator;
    use maji::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    // 1. Hardware init
    maji::init();

    // 2. Memory & heap
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    // 3. Boot screen + shell prompt
    maji::vga_buffer::clear_screen();
    maji::shell::SHELL.lock().prompt();

    #[cfg(test)]
    test_main();

    // 4. Start async executor
    let mut executor = maji::task::executor::Executor::new();
    executor.spawn(maji::task::Task::new(maji::task::keyboard::print_keypresses()));
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    maji::println!("{}", info);
    maji::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    maji::test_panic_handler(info)
}