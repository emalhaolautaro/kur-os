//! # Manejo de Interrupciones
//!
//! Este módulo configura la IDT (Interrupt Descriptor Table) y define los handlers
//! para las excepciones del CPU.
//!
//! ## ¿Qué es la IDT?
//! La IDT es una tabla que mapea cada número de interrupción/excepción a su handler.
//! Cuando ocurre una interrupción (ej: división por cero, breakpoint, page fault),
//! el CPU busca en la IDT el handler correspondiente y lo ejecuta.
//!
//! ## Excepciones vs Interrupciones
//! - **Excepciones**: Generadas por el CPU (errores, breakpoints, page faults)
//! - **Interrupciones**: Generadas por hardware externo (teclado, timer, disco)
//!
//! ## Nota sobre SSE y soft-float
//! Los handlers usan stacks dedicados de la IST (ver gdt.rs) para evitar problemas
//! de alineación. Además, el kernel usa `rustc-abi: x86-softfloat` para evitar
//! que el compilador genere instrucciones SSE que podrían fallar en contexto
//! de interrupción.

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;

lazy_static! {
    /// Interrupt Descriptor Table estática.
    /// 
    /// Configura los handlers para:
    /// - Breakpoint (int 3): Usado para debugging
    /// - Double Fault: Excepción crítica cuando falla el manejo de otra excepción
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            // Breakpoint: usa stack IST dedicado para evitar problemas de alineación
            idt.breakpoint
                .set_handler_fn(breakpoint_handler)
                .set_stack_index(crate::gdt::BREAKPOINT_IST_INDEX);
            
            // Double Fault: DEBE usar stack IST separado
            // Si el stack principal está corrupto, sin IST habría triple fault
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt
    };
}

/// Carga la IDT en el registro IDTR del CPU.
/// 
/// Debe llamarse después de `gdt::init()` ya que los handlers usan stacks del TSS.
pub fn init_idt() {
    IDT.load();
}

/// Handler para la excepción Breakpoint (vector 3).
/// 
/// Se dispara cuando se ejecuta la instrucción `int3`.
/// Usado para debugging - el programa puede continuar después.
/// 
/// # Argumentos
/// - `stack_frame`: Contiene el estado del CPU al momento de la excepción
///   (instruction pointer, stack pointer, flags, etc.)
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::println!("--- EXCEPCION: BREAKPOINT ---");
    crate::serial_println!("--- EXCEPCION: BREAKPOINT ---");
    crate::serial_println!("Stack Frame: {:#?}", stack_frame);
    // El handler retorna, permitiendo que el programa continúe
}

/// Handler para la excepción Double Fault (vector 8).
/// 
/// Se dispara cuando ocurre una excepción mientras se maneja otra excepción.
/// Es una excepción crítica que indica un problema grave en el kernel.
/// 
/// # Divergente
/// Este handler nunca retorna (`-> !`) porque no hay forma segura de recuperarse
/// de un double fault. Llamamos a `panic!` para mostrar información útil.
/// 
/// # Argumentos
/// - `stack_frame`: Estado del CPU al momento del fallo
/// - `_error_code`: Siempre 0 para double fault
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, 
    _error_code: u64
) -> ! {
    panic!("EXCEPCIÓN: DOBLE FALLO\n{:#?}", stack_frame);
}