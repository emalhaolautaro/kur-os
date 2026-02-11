#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kur_os::println;
use bootloader::{BootInfo, entry_point};

extern crate alloc;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kur_os::memory;
    use kur_os::allocator;
    use kur_os::task::{Task, executor::Executor, keyboard};
    use x86_64::VirtAddr;

    println!("Hola desde el kernel!");
    kur_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    unsafe {
        memory::init(phys_mem_offset, &boot_info.memory_map);
    }

    println!("Memoria inicializada correctamente.");

    allocator::init_heap().expect("falló la inicialización del heap");

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("número async: {}", number);
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kur_os::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kur_os::test_panic_handler(info)
}