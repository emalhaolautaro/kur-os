//! # kur-os - Kernel Educativo en Rust
//!
//! Este es el módulo principal de la biblioteca del kernel. Expone los subsistemas
//! y funcionalidades comunes a todo el kernel.
//!
//! ## Arquitectura
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                    main.rs                          │
//! │              (punto de entrada)                     │
//! └─────────────────────┬───────────────────────────────┘
//!                       │
//!                       ▼
//! ┌─────────────────────────────────────────────────────┐
//! │                    lib.rs                           │
//! │           (inicialización y utilities)              │
//! └───────┬─────────────┬────────────────┬──────────────┘
//!         │             │                │
//!         ▼             ▼                ▼
//!    ┌─────────┐   ┌──────────┐   ┌─────────────┐
//!    │  gdt.rs │   │interrupts│   │ vga_buffer  │
//!    │  (GDT)  │   │  (IDT)   │   │  (pantalla) │
//!    └─────────┘   └──────────┘   └─────────────┘
//! ```
//!
//! ## Características del Target
//! Este kernel usa un target custom (`x86_64-kur_os.json`) con:
//! - `rustc-abi: x86-softfloat` - Evita instrucciones SSE en handlers de interrupción
//! - `disable-redzone: true` - Necesario para código de kernel
//! - `panic-strategy: abort` - No hay stack unwinding

#![feature(abi_x86_interrupt)]  // Habilita el ABI especial para handlers de interrupción
#![no_std]                       // No usamos la biblioteca estándar
#![cfg_attr(test, no_main)]      // En modo test, no hay main normal
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

/// Módulo para comunicación serial (COM1).
/// Permite imprimir mensajes al host durante el desarrollo.
#[macro_use]
pub mod serial;

/// Módulo para el buffer VGA en modo texto.
/// Permite imprimir en la pantalla del sistema.
#[macro_use]
pub mod vga_buffer;

/// Global Descriptor Table y Task State Segment.
/// Configura los segmentos de memoria y los stacks para excepciones.
pub mod gdt;

/// Interrupt Descriptor Table.
/// Configura los handlers para excepciones e interrupciones.
pub mod interrupts;

/// Trait para funciones de test que pueden ejecutarse automáticamente.
pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// Inicializa todos los subsistemas del kernel.
/// 
/// Debe llamarse al inicio de `_start()` antes de cualquier otra operación.
/// 
/// # Orden de inicialización
/// 1. GDT y TSS - Necesarios para que funcionen las interrupciones
/// 2. IDT - Configura los handlers de excepciones
pub fn init() {
    gdt::init();
    interrupts::init_idt();
}

/// Ejecuta todos los tests y sale de QEMU con el código apropiado.
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Ejecutando {} pruebas", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// Handler de panic para modo test.
/// Imprime el error y sale de QEMU con código de fallo.
pub fn test_panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[fallido]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

/// Punto de entrada para `cargo test`.
/// En modo test, los tests de integración usan su propio _start.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    test_main();
    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test_panic_handler(info)
}

/// Códigos de salida para QEMU.
/// Usados para indicar éxito o fallo en tests automatizados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    /// Test exitoso (se mapea a código de salida 33 en QEMU)
    Success = 0x10,
    /// Test fallido (se mapea a código de salida 35 en QEMU)
    Failed = 0x11,
}

/// Sale de QEMU escribiendo al puerto de debug.
/// 
/// Esto funciona porque QEMU está configurado con:
/// `-device isa-debug-exit,iobase=0xf4,iosize=0x04`
/// 
/// El código de salida real es `(exit_code << 1) | 1`
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}