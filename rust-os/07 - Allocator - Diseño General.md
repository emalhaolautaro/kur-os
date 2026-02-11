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

## Expansión Dinámica del Heap

Originalmente, el heap tenía un tamaño fijo. Ahora soporta crecimiento bajo demanda:

### Lógica en `alloc`

Cuando `SlabAllocator` o `BuddyAllocator` se quedan sin memoria (`alloc` retorna `null`):

1. **Calcula el tamaño necesario**: `max(layout.size, layout.align)` redondeado a la siguiente potencia de dos (mínimo 4KB).
2. **Solicita páginas físicas**: Llama a `memory::map_page` para mapear el nuevo rango de direcciones virtuales (inmediatamente después del final actual del heap).
3. **Añade memoria**: Llama a `allocator.add_memory(start, size)` para informar al BuddyAllocator del nuevo bloque disponible.
4. **Reintenta**: Vuelve a intentar la asignación, que ahora debería tener éxito.

```rust
if ptr.is_null() {
    // ... calcular nuevo bloque ...
    // ... map_page loop ...
    if mapping_success {
        allocator.add_memory(current_end, block_size);
        ptr = allocator.allocate(...);
    }
}
```

---

## `init_heap` — Inicialización del heap

Esta función tiene dos responsabilidades:

### 1. Mapear el heap inicial

Itera sobre todas las páginas del rango inicial `[HEAP_START, HEAP_START + HEAP_SIZE)` y las mapea usando `memory::map_page`.

### 2. Inicializar el Buddy+Slab

```rust
ALLOCATOR.init(HEAP_START, HEAP_SIZE);
```

Le dice al `SlabAllocator` (y al `BuddyAllocator` interno) dónde empieza y cuánto mide la memoria inicial. A partir de aquí, el heap puede crecer más allá de `HEAP_SIZE`.

---

## Flujo completo de una asignación

```
Box::new(42)
  └─ GlobalAlloc::alloc(Layout { size: 4, align: 4 })
       └─ without_interrupts
            └─ SlabAllocator::allocate(size=4, align=4)
                 └─ ...
                 └─ si falla (OOM) → Expandir Heap
                      └─ memory::map_page(new_pages)
                      └─ BuddyAllocator::add_memory(new_block)
                      └─ Reintentar asignación
```

---

## Nota sobre `linked_list_allocator`

La dependencia `linked_list_allocator` en `Cargo.toml` es un remanente de una implementación anterior. Fue reemplazada por el sistema Buddy+Slab actual y puede removerse en el futuro.
