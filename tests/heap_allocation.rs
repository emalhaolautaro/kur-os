#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

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

use alloc::boxed::Box;

#[test_case]
fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

use alloc::vec::Vec;

#[test_case]
fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

use kur_os::allocator::HEAP_SIZE;

#[test_case]
fn many_boxes() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn fragmentation() {
    use alloc::vec::Vec;

    let mut blocks: Vec<Box<[u8; 16]>> = Vec::new();
    for _ in 0..1000 {
        blocks.push(Box::new([0u8; 16]));
    }

    let mut i = 0;
    blocks.retain(|_| {
        let keep = i % 2 != 0;
        i += 1;
        keep
    });

    assert_eq!(blocks.len(), 500);

    let mut larger_blocks: Vec<Box<[u8; 32]>> = Vec::new();
    for _ in 0..500 {
        larger_blocks.push(Box::new([1u8; 32]));
    }

    assert_eq!(larger_blocks.len(), 500);
}
