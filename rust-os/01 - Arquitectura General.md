# Arquitectura General

## Target personalizado (`x86_64-kur_os.json`)

El kernel compila para un target bare-metal custom definido en `x86_64-kur_os.json`. Decisiones clave:

| Campo | Valor | Razón |
|-------|-------|-------|
| `llvm-target` | `x86_64-unknown-none` | Sin sistema operativo subyacente |
| `os` | `"none"` | Bare metal |
| `linker` | `rust-lld` | Linker de LLVM, multiplataforma |
| `panic-strategy` | `"abort"` | Sin stack unwinding (no hay runtime) |
| `disable-redzone` | `true` | La red zone de System V ABI es peligrosa en kernels porque las interrupciones la pueden corromper |
| `rustc-abi` | `x86-softfloat` | Sin instrucciones SSE/AVX de hardware |
| `features` | `-mmx,-sse,-sse2,+soft-float` | Deshabilitar SIMD; las interrupciones no guardan registros SSE |
| `stack-probes` | `"none"` | Sin soporte de stack probes en bare metal |

### ¿Por qué deshabilitar SSE?

En un kernel, las interrupciones pueden ocurrir en cualquier momento. Si el kernel usa registros SSE y una interrupción no los guarda/restaura, se corrompen silenciosamente. Usar `soft-float` evita este problema por completo.

### ¿Por qué deshabilitar la Red Zone?

La **red zone** es un área de 128 bytes por debajo del stack pointer que funciones hoja pueden usar sin ajustar `rsp`. Cuando ocurre una interrupción, el CPU escribe el stack frame de interrupción directamente sobre la red zone, corrompiendo datos del kernel. Deshabilitarla es obligatorio en código de kernel.

---

## Estructura de la crate

El proyecto es tanto un binario (`main.rs`) como una librería (`lib.rs`). Esta dualidad permite:

1. **`main.rs`** → Punto de entrada real del kernel (`kernel_main`)
2. **`lib.rs`** → Expone módulos y funciones para los tests de integración

### Módulos declarados en `lib.rs`

```
lib.rs
├── serial       (macro_use)  → serial_print!, serial_println!
├── vga_buffer   (macro_use)  → print!, println!
├── gdt          → Global Descriptor Table + TSS
├── interrupts   → IDT + handlers
├── memory       → Paginación + frame allocator
├── buddy        → Buddy Allocator
├── slab         → Slab Allocator
└── allocator    → Integración GlobalAlloc + init_heap
```

### Función `init()`

Centraliza la inicialización del hardware en un orden específico:

```rust
pub fn init() {
    gdt::init();                              // 1. Cargar GDT + TSS
    interrupts::init_idt();                   // 2. Cargar IDT
    unsafe { interrupts::PICS.lock().initialize() }; // 3. Inicializar PIC
    x86_64::instructions::interrupts::enable(); // 4. Habilitar interrupciones
}
```

> **Orden importante:** La GDT debe cargarse antes que la IDT porque los handlers de interrupción usan los stacks definidos en la TSS (parte de la GDT).

---

## `kernel_main` — Punto de entrada

Después de que el bootloader cede control:

1. Imprime mensaje por VGA
2. Llama a `init()` (GDT → IDT → PIC → interrupts)
3. Configura paginación con `memory::init()`
4. Crea `BootInfoFrameAllocator` desde el memory map del bootloader
5. Inicializa el heap con `allocator::init_heap()`
6. Ejecuta validaciones de heap (Box, Vec, Rc) como smoke test
7. Entra en `hlt_loop()` — loop infinito usando la instrucción `hlt`

### `hlt_loop()`

```rust
pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
```

Usa `hlt` en vez de un `loop {}` vacío para que la CPU entre en estado de bajo consumo entre interrupciones, en lugar de quemar ciclos.

---

## Bootloader

Se usa `bootloader` v0.9 con el feature `map_physical_memory`. Este feature le indica al bootloader que mapee **toda la memoria física** en el espacio virtual del kernel con un offset fijo (`physical_memory_offset`). Esto simplifica enormemente el manejo de page tables porque cualquier dirección física `P` se puede acceder como dirección virtual `P + offset`.

El macro `entry_point!(kernel_main)` valida en tiempo de compilación que la firma de `kernel_main` sea correcta: `fn(&'static BootInfo) -> !`.

---

## Configuración de QEMU

Definida en `Cargo.toml` bajo `[package.metadata.bootimage]`:

- **Ejecución normal:** `-serial stdio -accel tcg`
  - Redirige el puerto serie a la terminal
  - Usa el acelerador TCG (emulación pura, sin KVM)
- **Tests:** agrega `-device isa-debug-exit`, `-display none`
  - El dispositivo `isa-debug-exit` permite que el kernel salga de QEMU enviando un código al puerto `0xf4`
  - `test-success-exit-code = 33` → QEMU retorna `(0x10 << 1) | 1 = 33` para éxito
