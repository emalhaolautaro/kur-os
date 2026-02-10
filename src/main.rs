#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kur_os::println;
use bootloader::{BootInfo, entry_point};

extern crate alloc;

use alloc::{boxed::Box, vec, vec::Vec, rc::Rc};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use kur_os::memory;
    use kur_os::allocator;
    use x86_64::VirtAddr;

    println!("Hola desde el kernel!");
    kur_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        memory::BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    println!("Memoria inicializada correctamente.");

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("falló la inicialización del heap");

    let heap_value = Box::new(41);
    println!("valor del heap en {:p}", heap_value);

    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vector en {:p}", vec.as_slice());

    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!("el conteo de referencia actual es {}", Rc::strong_count(&cloned_reference));
    core::mem::drop(reference_counted);
    println!("el conteo de referencia ahora es {} ahora", Rc::strong_count(&cloned_reference));

    #[cfg(test)]
    test_main();

    println!("¡No se cayó!");
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