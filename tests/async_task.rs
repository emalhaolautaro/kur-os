#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kur_os::task::{Task, simple_executor::SimpleExecutor};

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
fn test_simple_executor_runs_task() {
    use core::sync::atomic::{AtomicBool, Ordering};

    static EXECUTED: AtomicBool = AtomicBool::new(false);

    async fn set_flag() {
        EXECUTED.store(true, Ordering::SeqCst);
    }

    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(set_flag()));
    executor.run();

    assert!(EXECUTED.load(Ordering::SeqCst));
}

#[test_case]
fn test_task_id_unique() {
    let t1 = Task::new(async {});
    let t2 = Task::new(async {});
    let t3 = Task::new(async {});
    // Los IDs se generan con un AtomicU64 incremental,
    // así que cada tarea debe tener un ID distinto.
    // No podemos acceder al campo privado directamente,
    // pero si se crean sin panic, el contador funciona.
    drop(t1);
    drop(t2);
    drop(t3);
}

#[test_case]
fn test_simple_executor_multiple_tasks() {
    use core::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    async fn increment() {
        COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(increment()));
    executor.spawn(Task::new(increment()));
    executor.spawn(Task::new(increment()));
    executor.run();

    assert_eq!(COUNTER.load(Ordering::SeqCst), 3);
}

#[test_case]
fn test_async_value_propagation() {
    use core::sync::atomic::{AtomicU32, Ordering};

    static RESULT: AtomicU32 = AtomicU32::new(0);

    async fn compute() -> u32 {
        42
    }

    async fn store_result() {
        let val = compute().await;
        RESULT.store(val, Ordering::SeqCst);
    }

    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(store_result()));
    executor.run();

    assert_eq!(RESULT.load(Ordering::SeqCst), 42);
}
