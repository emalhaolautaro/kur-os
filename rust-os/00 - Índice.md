# 🦀 kur-os — Documentación del Kernel

> Sistema operativo educativo escrito en Rust, orientado a la arquitectura x86_64.
> Corre sobre QEMU usando el crate `bootloader` v0.9.

---

## Mapa del proyecto

| Sección | Tema | Archivos clave |
|---------|------|----------------|
| [[01 - Arquitectura General]] | Target custom, boot, estructura de crate | `x86_64-kur_os.json`, `Cargo.toml`, `main.rs`, `lib.rs` |
| [[02 - VGA Buffer]] | Salida por pantalla en modo texto | `vga_buffer.rs` |
| [[03 - Puerto Serie]] | Comunicación UART para debugging y tests | `serial.rs` |
| [[04 - GDT y TSS]] | Segmentación, stacks de interrupción (IST) | `gdt.rs` |
| [[05 - Interrupciones]] | IDT, PIC 8259, handlers de CPU y hardware | `interrupts.rs` |
| [[06 - Memoria y Paginación]] | Page tables, traducción de direcciones, frame allocator | `memory.rs` |
| [[07 - Allocator - Diseño General]] | Estrategia híbrida Buddy+Slab, integración con `GlobalAlloc` | `allocator.rs` |
| [[08 - Buddy Allocator]] | Asignador "mayorista" por potencias de 2 | `buddy.rs` |
| [[09 - Slab Allocator]] | Caches de tamaño fijo para objetos pequeños | `slab.rs` |
| [[10 - Testing]] | Framework de tests, QEMU, tests de integración | `tests/` |
| [[11 - Async Await]] | Multitarea cooperativa, executor con wakers, teclado async | `task/` |

---

## Flujo de arranque (resumen)

```
bootloader → kernel_main()
  ├─ init()
  │   ├─ gdt::init()          → GDT + TSS + segmentos
  │   ├─ interrupts::init_idt() → IDT con handlers
  │   ├─ PICS.initialize()     → PIC 8259 remapeado
  │   └─ interrupts::enable()  → habilitar interrupciones
  ├─ memory::init()            → OffsetPageTable
  ├─ BootInfoFrameAllocator    → marcos físicos
  ├─ allocator::init_heap()    → mapear heap + Buddy+Slab
  └─ Executor::run()           → tareas async (teclado, etc.)
```

---

## Dependencias principales

| Crate | Versión | Propósito |
|-------|---------|-----------|
| `bootloader` | 0.9 | Carga del kernel, `map_physical_memory` |
| `x86_64` | 0.14.2 | Estructuras de CPU (IDT, GDT, paginación) |
| `volatile` | 0.2.6 | Escrituras volátiles al VGA buffer |
| `spin` | 0.9.8 | Mutex sin bloqueo (spinlock) |
| `lazy_static` | 1.4.0 | Estáticas inicializadas en runtime (`spin_no_std`) |
| `pic8259` | 0.11.0 | Controlador PIC encadenado |
| `uart_16550` | 0.3.0 | Puerto serie UART |
| `pc-keyboard` | 0.8.0 | Decodificación de scancodes |
| `crossbeam-queue` | 0.3 | Cola lock-free (`ArrayQueue`) para scancodes |
| `conquer-once` | 0.4 | `OnceCell` para `no_std` |
| `futures-util` | 0.3 | Traits `Stream`, `StreamExt`, `AtomicWaker` |

> **Nota:** `linked_list_allocator` aparece como dependencia pero actualmente no se usa; fue reemplazado por la implementación propia Buddy+Slab.
