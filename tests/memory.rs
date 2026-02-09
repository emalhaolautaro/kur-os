#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use bootloader::{BootInfo, entry_point};
use kur_os::serial_println;

entry_point!(main);

fn main(_boot_info: &'static BootInfo) -> ! {
    kur_os::init();
    test_main();
    kur_os::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kur_os::test_panic_handler(info)
}

// ============================================================================
// Tests de memoria y paginación
// ============================================================================

#[test_case]
fn test_vga_buffer_is_mapped() {
    // El buffer VGA siempre debe estar mapeado en 0xb8000
    let vga_ptr = 0xb8000 as *mut u8;
    
    // Si podemos escribir y leer del VGA buffer, está mapeado
    unsafe {
        let original = vga_ptr.read_volatile();
        vga_ptr.write_volatile(original);
    }
    
    serial_println!("[ok]");
}

#[test_case]
fn test_kernel_code_is_accessible() {
    // La dirección de esta función debe ser accesible
    fn dummy() {}
    let fn_ptr = dummy as *const ();
    assert!(!fn_ptr.is_null());
    serial_println!("[ok]");
}

#[test_case]
fn test_stack_is_accessible() {
    // Podemos escribir en el stack
    let mut stack_var: u64 = 0xDEADBEEF;
    let ptr = &mut stack_var as *mut u64;
    unsafe {
        ptr.write_volatile(0x12345678);
        let read_back = ptr.read_volatile();
        assert_eq!(read_back, 0x12345678);
    }
    serial_println!("[ok]");
}
