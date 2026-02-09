#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kur_os::println;
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kur_os::memory;
    use x86_64::VirtAddr;

    println!("Hola desde el kernel!");
    kur_os::init();

    // Inicializar el subsistema de memoria
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let _mapper = unsafe { memory::init(phys_mem_offset) };
    let _frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    println!("Memoria inicializada correctamente.");

    #[cfg(test)]
    test_main();

    kur_os::hlt_loop();
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