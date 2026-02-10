# Slab Allocator

> **Archivo:** `src/slab.rs`
> **Propósito:** Asignador "minorista" que maneja objetos pequeños (≤ 2048 bytes) usando caches de tamaño fijo. Obtiene páginas del [[08 - Buddy Allocator|Buddy Allocator]].

---

## Concepto

El Buddy Allocator tiene un mínimo de 4KB. Si pedís 16 bytes, desperdiciás ~4080. El Slab resuelve esto:

1. Pide una página de 4KB al Buddy
2. La divide en objetos de **tamaño fijo** (ej: 16 bytes → ~253 objetos)
3. Asignar/liberar un objeto es O(1)

---

## Tamaños de cache

```rust
const CACHE_SIZES: [usize; 9] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];
```

Cuando se pide N bytes, se redondea al cache más cercano. Todo lo que supere 2048 va directo al Buddy.

---

## Estructuras

### `FreeObject`

Nodo de free list que vive dentro del objeto libre (sin overhead de metadatos):

```rust
#[repr(C)]
struct FreeObject {
    next: Option<ptr::NonNull<FreeObject>>,
}
```

### `Slab`

Una página de 4KB dividida en objetos iguales:

```rust
struct Slab {
    next: Option<ptr::NonNull<Slab>>,
    free_list: Option<ptr::NonNull<FreeObject>>,
    free_count: usize,
    object_size: usize,
}
```

**Layout en memoria:**

```
┌──────────────┬─────────┬────────┬────────┬─────┬────────┐
│ Header(Slab) │ padding │ Obj 0  │ Obj 1  │ ... │ Obj N  │
└──────────────┴─────────┴────────┴────────┴─────┴────────┘
← PAGE_SIZE (4096 bytes) ────────────────────────────────→
```

El header vive al inicio de la página. Los objetos comienzan alineados al `object_size`.

### `SlabCache`

Agrupa slabs de un mismo tamaño en dos listas:

```rust
struct SlabCache {
    partial_slabs: Option<ptr::NonNull<Slab>>,  // tienen espacio
    full_slabs: Option<ptr::NonNull<Slab>>,     // completamente ocupados
    object_size: usize,
}
```

**¿Por qué dos listas?** Los `partial_slabs` se buscan primero para asignar. Los `full_slabs` se ignoran. Cuando se libera un objeto de un slab full, se mueve a partial.

---

## Operaciones

### `SlabCache::allocate(buddy)`

1. ¿Hay slabs parciales? → asignar objeto de ahí
   - Si el slab quedó lleno → moverlo a `full_slabs`
2. No hay → pedir página al Buddy → `Slab::init()` → asignar primer objeto

### `SlabCache::deallocate(ptr)`

1. Encontrar el slab: `slab_addr = ptr & !(PAGE_SIZE - 1)` (redondear a página)
2. Si estaba full → mover de `full_slabs` a `partial_slabs`
3. Devolver objeto a la free list del slab

> **Truco:** Como el header vive al inicio de la página alineada a 4KB, encontrar el slab es una simple operación AND.

---

## `SlabAllocator` — Orquestación

```rust
pub struct SlabAllocator {
    caches: [SlabCache; 9],
    buddy: BuddyAllocator,
}
```

### Lógica de despacho

```rust
let effective_size = size.max(align);
if effective_size <= 2048 {
    // → SlabCache apropiado (find_cache_index)
} else {
    // → BuddyAllocator directamente
}
```

Usa `max(size, align)` porque el alignment puede requerir un cache mayor. Ejemplo: 4 bytes con align 32 → cache de 32.

---

## Safety

`SlabAllocator` implementa `Send` manualmente porque gestiona punteros raw internos a memoria del heap sin aliasing.
