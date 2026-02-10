#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kur_os::task::{Task, simple_executor::SimpleExecutor, stress_test};

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use kur_os::allocator;
    use kur_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    kur_os::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("falló la inicialización del heap");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kur_os::test_panic_handler(info)
}

#[test_case]
fn test_heap_stress() {
    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(stress_test::heap_stress_test()));
    executor.run();
}
