//! # Punto de Entrada del Kernel
//!
//! Este archivo contiene la función `_start`, el punto de entrada del kernel
//! cuando el bootloader le transfiere el control.

#![no_std]   // No usamos la biblioteca estándar (no hay OS debajo)
#![no_main]  // No usamos el runtime estándar de Rust (no hay main normal)
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kur_os::println;

/// Punto de entrada del kernel.
/// 
/// Esta función es llamada por el bootloader después de:
/// 1. Configurar el modo protegido de 64 bits
/// 2. Configurar una GDT e IDT mínimas
/// 3. Configurar paginación de identidad para los primeros MB
/// 
/// # Importante
/// - `#[unsafe(no_mangle)]` evita que Rust cambie el nombre de la función
/// - `extern "C"` usa la convención de llamada de C
/// - `-> !` indica que la función nunca retorna (divergente)
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hola");

    // Inicializar subsistemas del kernel (GDT, IDT)
    kur_os::init();

    println!("Probando interrupciones");

    // Generar una excepción de breakpoint para probar el handler
    // Si las interrupciones están bien configuradas, el programa continúa
    x86_64::instructions::interrupts::int3();

    // Solo en modo test: ejecutar los tests
    #[cfg(test)]
    test_main();

    println!("No se bloqueo");

    // El kernel nunca termina - loop infinito
    loop {}
}

/// Handler de panic para modo normal (no test).
/// 
/// Cuando ocurre un panic, mostramos el mensaje en pantalla y entramos
/// en un loop infinito (halt sería mejor, pero por ahora esto funciona).
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

/// Handler de panic para modo test.
/// 
/// Delega al handler de la biblioteca que imprime por serial y sale de QEMU.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kur_os::test_panic_handler(info)
}