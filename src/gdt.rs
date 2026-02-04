//! # Global Descriptor Table (GDT)
//!
//! Este módulo configura la GDT y el TSS (Task State Segment) necesarios para el
//! funcionamiento del kernel en modo protegido de 64 bits.
//!
//! ## ¿Qué es la GDT?
//! La GDT es una estructura de datos usada por procesadores x86 para definir las
//! características de los segmentos de memoria. En modo largo (64-bit), la segmentación
//! está mayormente deshabilitada, pero la GDT sigue siendo necesaria para:
//! - Definir los segmentos de código y datos del kernel
//! - Cargar el TSS (necesario para cambiar stacks en interrupciones)
//!
//! ## ¿Qué es el TSS?
//! El Task State Segment contiene la Interrupt Stack Table (IST), que permite usar
//! stacks alternativos para manejar excepciones críticas como double faults.
//! Esto evita triple faults cuando el stack original está corrupto.

use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use lazy_static::lazy_static;
use x86_64::VirtAddr;

/// Índice en la IST para el handler de double fault.
/// Usar un stack separado evita triple faults si el stack principal está corrupto.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Índice en la IST para el handler de breakpoint.
/// Stack separado con alineación de 16 bytes para compatibilidad.
pub const BREAKPOINT_IST_INDEX: u16 = 1;

lazy_static! {
    /// Task State Segment estático.
    /// Contiene los stacks alternativos para manejo de excepciones.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        
        // Stack dedicado para double fault (20 KB, alineado a 16 bytes)
        // Se usa cuando ocurre una excepción mientras se maneja otra excepción
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5; // 20 KB
            #[repr(align(16))]
            #[allow(dead_code)]
            struct AlignedStack([u8; 4096 * 5]);
            static mut STACK: AlignedStack = AlignedStack([0; 4096 * 5]);

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            // El stack crece hacia abajo, así que devolvemos el TOP (inicio + tamaño)
            stack_start + STACK_SIZE as u64
        };
        
        // Stack dedicado para breakpoint (20 KB, alineado a 16 bytes)
        tss.interrupt_stack_table[BREAKPOINT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            #[repr(align(16))]
            #[allow(dead_code)]
            struct AlignedStack([u8; 4096 * 5]);
            static mut STACK: AlignedStack = AlignedStack([0; 4096 * 5]);

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE as u64
        };
        
        tss
    };
}

lazy_static! {
    /// Global Descriptor Table y sus selectores.
    /// 
    /// Contiene:
    /// - Segmento de código del kernel (CS) - DPL 0
    /// - Segmento de datos del kernel (SS) - DPL 0  
    /// - Descriptor del TSS
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Segmento de código: permite ejecutar instrucciones en ring 0
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        
        // Segmento de datos: permite acceso a memoria en ring 0
        // IMPORTANTE: Debe ser kernel_data_segment (DPL=0), no user_data_segment
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment()); 
        
        // TSS: necesario para que el CPU sepa dónde están los stacks de la IST
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        
        (gdt, Selectors { code_selector, data_selector, tss_selector })
    };
}

/// Selectores de segmento usados para configurar los registros CS, SS y TR.
struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Inicializa la GDT y carga los selectores en los registros del CPU.
/// 
/// Esta función debe llamarse temprano en el arranque, antes de habilitar interrupciones.
/// 
/// # Pasos:
/// 1. Carga la GDT en el registro GDTR
/// 2. Configura CS (Code Segment) con el selector de código del kernel
/// 3. Configura SS (Stack Segment) con el selector de datos del kernel
/// 4. Carga el TSS en el registro TR (Task Register)
pub fn init() {
    use x86_64::registers::segmentation::{CS, Segment, SS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        SS::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
}