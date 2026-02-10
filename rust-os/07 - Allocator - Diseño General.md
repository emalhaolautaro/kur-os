# Allocator — Diseño General

> **Archivo:** `src/allocator.rs`
> **Propósito:** Integrar el sistema Buddy+Slab como allocator global del kernel e inicializar la región de heap.

---

## Estrategia híbrida: Mayorista y Minorista

El allocator usa una arquitectura de dos niveles inspirada en el allocator del kernel Linux:

```
┌──────────────────────────────────────────────────┐
│         GlobalAlloc (interfaz de Rust)            │
├──────────────────────────────────────────────────┤
│              SlabAllocator                        │
│  ┌─────────────────────────────────────────────┐ │
│  │ Objetos ≤ 2048 bytes → Slab Caches          │ │
│  │ (8, 16, 32, 64, 128, 256, 512, 1024, 2048) │ │
│  └─────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────┐ │
│  │ Objetos > 2048 bytes → BuddyAllocator       │ │
│  │ (bloques de 4KB en adelante)                │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

- **Slab** ("minorista"): Eficiente para objetos pequeños y frecuentes. Sin fragmentación interna significativa.
- **Buddy** ("mayorista"): Maneja bloques grandes y provee páginas al Slab cuando necesita más memoria.

---

## Configuración del heap

```rust
pub const HEAP_SIZE: usize = 128 * 1024;          // 128 KB
pub const HEAP_START: usize = 0x_4444_4442_0000;   // dirección virtual fija
```

### Restricciones validadas en `init_heap`

```rust
assert!(HEAP_START % HEAP_SIZE == 0);   // alineado al tamaño
assert!(HEAP_SIZE.is_power_of_two());    // potencia de 2 (para Buddy)
assert!(HEAP_SIZE >= PAGE_SIZE);         // al menos una página
```

Estas restricciones son necesarias para el correcto funcionamiento del Buddy Allocator, que asume que la memoria comienza alineada a su tamaño total.

---

## `LockedSlabAllocator`

Wrapper thread-safe alrededor de `SlabAllocator`:

```rust
pub struct LockedSlabAllocator {
    inner: Mutex<SlabAllocator>,
}
```

### Implementación de `GlobalAlloc`

```rust
unsafe impl GlobalAlloc for LockedSlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        interrupts::without_interrupts(|| {
            self.inner.lock().allocate(layout.size(), layout.align())
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        interrupts::without_interrupts(|| {
            self.inner.lock().deallocate(ptr, layout.size(), layout.align())
        })
    }
}
```

**Puntos clave:**
- Usa `interrupts::without_interrupts` para evitar deadlocks (si una interrupción intenta asignar memoria mientras el allocator está bloqueado)
- Pasa tanto `size` como `align` al `SlabAllocator`, que usa `max(size, align)` para elegir el cache correcto

### Instancia global

```rust
#[global_allocator]
static ALLOCATOR: LockedSlabAllocator = LockedSlabAllocator::new();
```

---

## `init_heap` — Inicialización del heap

Esta función tiene dos responsabilidades:

### 1. Mapear páginas virtuales a frames físicos

```rust
for page in page_range {
    let frame = frame_allocator.allocate_frame()?;
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    mapper.map_to(page, frame, flags, frame_allocator)?.flush();
}
```

Itera sobre todas las páginas del rango `[HEAP_START, HEAP_START + HEAP_SIZE)` y las mapea a frames físicos obtenidos del `BootInfoFrameAllocator`.

### 2. Inicializar el Buddy+Slab

```rust
ALLOCATOR.init(HEAP_START, HEAP_SIZE);
```

Le dice al `SlabAllocator` (y al `BuddyAllocator` interno) dónde empieza y cuánto mide la memoria del heap.

---

## Flujo completo de una asignación

```
Box::new(42)
  └─ GlobalAlloc::alloc(Layout { size: 4, align: 4 })
       └─ without_interrupts
            └─ SlabAllocator::allocate(size=4, align=4)
                 └─ effective_size = max(4, 4) = 4
                 └─ 4 ≤ 2048 → buscar cache de 8 bytes
                      └─ SlabCache(8)::allocate()
                           └─ ¿hay slab parcial? → tomar objeto
                           └─ si no → BuddyAllocator::allocate(4096)
                                       → crear nuevo Slab
                                       → tomar primer objeto
```

---

## Nota sobre `linked_list_allocator`

La dependencia `linked_list_allocator` en `Cargo.toml` es un remanente de una implementación anterior. Fue reemplazada por el sistema Buddy+Slab actual y puede removerse en el futuro.
