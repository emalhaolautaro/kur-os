

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kur_os::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kur_os::println;


#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hola");

    kur_os::init();

    println!("Probando interrupciones");

    x86_64::instructions::interrupts::int3();

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