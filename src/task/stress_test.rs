use alloc::vec::Vec;
use crate::rng::SimpleRng;

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
        crate::serial_println!("=== Heap Stress Test — Resultados ===");
        crate::serial_println!("  Asignaciones:      {}", self.allocs);
        crate::serial_println!("  Liberaciones:      {}", self.deallocs);
        crate::serial_println!("  Bytes asignados:   {}", self.bytes_allocated);
        crate::serial_println!("  Bytes liberados:   {}", self.bytes_freed);
        crate::serial_println!("  Bytes en uso:      {}", self.bytes_allocated - self.bytes_freed);
        crate::serial_println!("  Pico de objetos:   {}", self.peak_objects);
    }
}

pub async fn heap_stress_test() {
    let mut rng = SimpleRng::new(42);
    let mut storage: Vec<Vec<u8>> = Vec::new();
    let mut stats = StressStats::new();

    crate::serial_println!("Iniciando Stress Test del Heap...");

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
            crate::serial_println!(
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
    crate::serial_println!("  Liberados {} objetos restantes", remaining);

    stats.print_summary();
    crate::serial_println!("Stress Test completado con éxito.");
}
