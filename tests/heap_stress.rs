#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kur_os::task::{Task, simple_executor::SimpleExecutor};
use kur_os::rng::SimpleRng;
use alloc::vec::Vec;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use kur_os::allocator;
    use kur_os::memory::{self, BootInfoFrameAllocator};
    use x86_64::VirtAddr;

    kur_os::init();
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    unsafe {
        memory::init(phys_mem_offset, &boot_info.memory_map);
    }
    allocator::init_heap().expect("falló la inicialización del heap");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kur_os::test_panic_handler(info)
}

struct StressStats {
    allocs: u64,
    deallocs: u64,
    bytes_allocated: u64,
    bytes_freed: u64,
    peak_objects: usize,
}

impl StressStats {
    fn new() -> Self {
        Self {
            allocs: 0,
            deallocs: 0,
            bytes_allocated: 0,
            bytes_freed: 0,
            peak_objects: 0,
        }
    }

    fn record_alloc(&mut self, size: usize, live_count: usize) {
        self.allocs += 1;
        self.bytes_allocated += size as u64;
        if live_count > self.peak_objects {
            self.peak_objects = live_count;
        }
    }

    fn record_dealloc(&mut self, size: usize) {
        self.deallocs += 1;
        self.bytes_freed += size as u64;
    }

    fn print_summary(&self) {
        kur_os::serial_println!("=== Heap Stress Test — Resultados ===");
        kur_os::serial_println!("  Asignaciones:      {}", self.allocs);
        kur_os::serial_println!("  Liberaciones:      {}", self.deallocs);
        kur_os::serial_println!("  Bytes asignados:   {}", self.bytes_allocated);
        kur_os::serial_println!("  Bytes liberados:   {}", self.bytes_freed);
        kur_os::serial_println!("  Bytes en uso:      {}", self.bytes_allocated - self.bytes_freed);
        kur_os::serial_println!("  Pico de objetos:   {}", self.peak_objects);
    }
}

async fn heap_stress_test() {
    let mut rng = SimpleRng::new(42);
    let mut storage: Vec<Vec<u8>> = Vec::new();
    let mut stats = StressStats::new();

    kur_os::serial_println!("Iniciando Stress Test del Heap...");

    for i in 0..5_000u64 {
        let action = rng.next_range(0, 10);

        if action < 7 && storage.len() < 50 {
            let size = rng.next_range(8, 256) as usize;
            let mut data = Vec::with_capacity(size);
            for _ in 0..size.min(10) {
                data.push(i as u8);
            }
            stats.record_alloc(size, storage.len() + 1);
            storage.push(data);
        } else if !storage.is_empty() {
            let removed = storage.remove(0);
            stats.record_dealloc(removed.capacity());
        }

        if i % 1000 == 0 {
            kur_os::serial_println!(
                "  Iteración {}: {} objetos en vuelo, {} bytes asignados",
                i,
                storage.len(),
                stats.bytes_allocated - stats.bytes_freed
            );
        }
    }

    let remaining = storage.len();
    for item in storage.drain(..) {
        stats.record_dealloc(item.capacity());
    }
    kur_os::serial_println!("  Liberados {} objetos restantes", remaining);

    stats.print_summary();
    kur_os::serial_println!("Stress Test completado con éxito.");
}

async fn heap_expansion_test() {
    kur_os::serial_println!("Iniciando Test de Expansión de Heap...");
    
    let size = 500 * 1024;
    let mut vec: Vec<u8> = Vec::with_capacity(size);
    
    for i in 0..size {
        vec.push((i % 255) as u8);
    }
    
    assert_eq!(vec.len(), size);
    assert_eq!(vec[size-1], ((size-1) % 255) as u8);
    
    kur_os::serial_println!("Vector de 500KB creado exitosamente. Heap expandido.");
}

#[test_case]
fn test_heap_stress() {
    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(heap_stress_test()));
    executor.spawn(Task::new(heap_expansion_test()));
    executor.run();
}
