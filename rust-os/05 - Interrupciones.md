# Interrupciones

> **Archivo:** `src/interrupts.rs`
> **Propósito:** Configurar la IDT, manejar excepciones del CPU y interrupciones de hardware (timer, teclado).

---

## IDT (Interrupt Descriptor Table)

La IDT es un array de 256 entradas donde cada entrada asocia un número de interrupción con un handler. Se inicializa con `lazy_static!` porque necesita referencias a funciones y configuración de IST.

### Handlers registrados

| # | Tipo | Handler | IST |
|---|------|---------|-----|
| 3 | Breakpoint | `breakpoint_handler` | IST 1 |
| 8 | Double fault | `double_fault_handler` | IST 0 |
| 14 | Page fault | `page_fault_handler` | — |
| 32 | Timer (IRQ0) | `timer_interrupt_handler` | — |
| 33 | Teclado (IRQ1) | `keyboard_interrupt_handler` | — |

---

## PIC 8259 (Programmable Interrupt Controller)

Los IRQs de hardware pasan por dos PICs encadenados (master + slave) antes de llegar al CPU. Por defecto, los PICs usan las líneas 0–15, que colisionan con las excepciones del CPU. Se remapean:

```rust
pub const PIC_1_OFFSET: u8 = 32;  // IRQ 0-7 → interrupciones 32-39
pub const PIC_2_OFFSET: u8 = 40;  // IRQ 8-15 → interrupciones 40-47
```

La instancia global del PIC se protege con `spin::Mutex`:

```rust
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
```

---

## Handlers de excepciones del CPU

### Breakpoint (INT 3)

```rust
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("--- EXCEPCION: BREAKPOINT ---");
    serial_println!("Stack Frame: {:#?}", stack_frame);
}
```

Imprime por VGA y por serial. No es fatal: la ejecución continúa después.

### Double Fault

```rust
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64
) -> ! {
    panic!("EXCEPCIÓN: DOBLE FALLO\n{:#?}", stack_frame);
}
```

Siempre fatal (retorna `!`). Usa la IST 0 para tener un stack limpio incluso si el stack original está corrupto.

### Page Fault

```rust
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!("Dirección Accedida: {:?}", Cr2::read());
    println!("Código de Error: {:?}", error_code);
    hlt_loop();
}
```

Lee el registro `CR2` para mostrar qué dirección virtual causó el fallo. Actualmente no recupera, entra en `hlt_loop()`.

---

## Handlers de hardware (IRQs)

### Timer (IRQ 0, interrupción 32)

```rust
extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Temporizador.as_u8()); }
}
```

Imprime un `.` en cada tick del timer. Envía EOI (End of Interrupt) al PIC para permitir futuras interrupciones.

### Teclado (IRQ 1, interrupción 33)

```rust
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Lee scancode del puerto 0x60
    // Decodifica con pc_keyboard (layout US104Key, ScancodeSet1)
    // Imprime el carácter o la tecla raw
    unsafe { PICS.lock().notify_end_of_interrupt(InterruptIndex::Teclado.as_u8()); }
}
```

Usa el crate `pc-keyboard` con:
- **Layout:** `Us104Key` (teclado americano estándar)
- **Scancode set:** `ScancodeSet1` (set legacy del 8042)
- **Control handling:** `HandleControl::Ignore`

El estado del teclado (`Keyboard`) se mantiene en un `lazy_static!` con `Mutex` dentro del propio handler.

---

## `InterruptIndex` (enum)

```rust
#[repr(u8)]
pub enum InterruptIndex {
    Temporizador = PIC_1_OFFSET,  // 32
    Teclado,                       // 33
}
```

Mapea nombres legibles a los números de interrupción remapeados. Implementa conversiones a `u8` y `usize` para indexar la IDT.

---

## Patrón: `notify_end_of_interrupt`

Cada handler de hardware **debe** enviar EOI al PIC después de procesarse. Sin esto, el PIC no entrega más interrupciones de esa línea. El orden es:

1. Procesar la interrupción
2. Enviar EOI

> **Cuidado:** No enviar EOI es un bug silencioso — el timer/teclado simplemente deja de funcionar.
