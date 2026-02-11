# Buddy Allocator

> **Archivo:** `src/buddy.rs`
> **Propósito:** Asignador "mayorista" que maneja bloques de memoria en potencias de 2 (desde 4KB hasta 128KB).

---

## Concepto

El Buddy Allocator divide la memoria en bloques cuyas tamaños son siempre potencias de 2. Cuando se necesita un bloque de cierto tamaño:

1. Si hay un bloque libre del tamaño exacto → entregarlo
2. Si no, tomar un bloque más grande y **dividirlo** (split) en dos mitades ("buddies")
3. Al liberar, intentar **fusionar** (coalesce) el bloque con su buddy si ambos están libres

La ventaja principal es que la fusión es O(1): el buddy de un bloque se encuentra con una simple operación XOR.

---

## Constantes

| Constante | Valor | Significado |
|-----------|-------|-------------|
| `PAGE_SIZE` | 4096 (4KB) | Tamaño mínimo de bloque |
| `MIN_ORDER` | 12 | 2^12 = 4096 bytes |
| `MAX_ORDER` | 21 | 2^21 = 2097152 bytes (2MB) |
| `NUM_ORDERS` | 10 | Órdenes disponibles: 12..21 |

---

## Estructuras

### `FreeBlock`

```rust
#[repr(C)]
struct FreeBlock {
    next: Option<ptr::NonNull<FreeBlock>>,
}
```

Nodo de lista enlazada que **vive dentro del bloque libre mismo**. Como los bloques libres no están en uso, reutilizamos sus primeros 8 bytes para almacenar el puntero al siguiente bloque libre. Esto evita overhead de metadatos.

### `BuddyAllocator`

```rust
pub struct BuddyAllocator {
    heap_start: usize,
    heap_size: usize,
    free_lists: [Option<ptr::NonNull<FreeBlock>>; NUM_ORDERS], // 6 listas
}
```

Mantiene una **free list** por cada orden. La free list del orden 12 tiene bloques de 4KB, la del orden 13 de 8KB, etc.

---

## El truco XOR para encontrar buddies

Dada la dirección de un bloque y su tamaño, el buddy se calcula como:

```rust
fn buddy_address(&self, addr: usize, block_size: usize) -> usize {
    self.heap_start + ((addr - self.heap_start) ^ block_size)
}
```

### ¿Por qué funciona?

Cuando dividimos un bloque de tamaño `2S` en la dirección `A`, los dos buddies quedan en:
- Buddy 0: dirección `A` (offset relativo 0)
- Buddy 1: dirección `A + S` (offset relativo S)

En binario, la diferencia entre ambos es exactamente el bit correspondiente al tamaño `S`. XOR con `S` voltea ese bit:
- Si tenemos Buddy 0 (bit = 0) → XOR da 1 → obtenemos Buddy 1
- Si tenemos Buddy 1 (bit = 1) → XOR da 0 → obtenemos Buddy 0

Se resta `heap_start` antes del XOR y se suma después para trabajar con offsets relativos al inicio del heap.

---

## Operaciones principales

### `add_memory(start, size)`

Añade un rango arbitrario de memoria al allocator.

1. Verifica la alineación de `start` con los órdenes de bloque.
2. Si `start` no está alineado para un orden grande, intenta con uno más pequeño.
3. Si ni siquiera está alineado a `MIN_ORDER`, avanza `start` por `PAGE_SIZE` hasta alinearse (resiliencia ante inputs desalineados).
4. Divide el rango en bloques del mayor orden posible y los libera (`free_block`) para que se integren a las listas.

### `allocate(size) -> *mut u8`

```
1. Ajustar size al mínimo de PAGE_SIZE
2. Calcular orden necesario (próxima potencia de 2)
3. Buscar bloque libre desde ese orden hasta MAX_ORDER
4. Si el bloque es más grande que lo necesario → split
5. Retornar la dirección del bloque
```

### `split_block(addr, current_order, target_order)`

```
Mientras current_order > target_order:
    order -= 1
    buddy_addr = addr + (1 << order)       ← mitad superior
    Agregar buddy a free_list[order]        ← el buddy queda libre
    Retener addr                            ← la mitad inferior se entrega
```

Divide recursivamente, siempre quedándose con la **mitad inferior** y liberando la **mitad superior** a la free list.

### `deallocate(ptr, size)`

```
1. Calcular orden del bloque
2. Llamar a free_block(ptr, order)
```

### `free_block(addr, order)` — Fusión recursiva

```
Mientras order < MAX_ORDER:
    buddy = buddy_address(addr, 1 << order)
    Si buddy está fuera del heap → break
    Si buddy NO está en la free list → break (no se puede fusionar)
    Remover buddy de la free list
    addr = min(addr, buddy)    ← nuevo bloque fusionado
    order += 1                 ← subir un nivel
Agregar el bloque (posiblemente fusionado) a free_list[order]
```

La fusión es **greedy**: intenta subir de orden lo más posible.

---

## Ejemplo visual

Estado inicial (heap de 128KB = orden 17):

```
free_lists[17-12=5]: [████████████████████████] 128KB
free_lists[4]:       vacía
free_lists[3]:       vacía
free_lists[2]:       vacía
free_lists[1]:       vacía
free_lists[0]:       vacía
```

Después de `allocate(4KB)` → se hace split en cadena:

```
free_lists[5]: vacía
free_lists[4]: [████████████] 64KB
free_lists[3]: [██████] 32KB
free_lists[2]: [███] 16KB
free_lists[1]: [██] 8KB
free_lists[0]: [En uso 4KB] [█ libre 4KB]
```

---

## Inicialización

```rust
pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
    // Verificar alineación a página
    // Encontrar el orden máximo que cabe
    // Crear un solo bloque libre del máximo orden
}
```

Comienza con toda la memoria como un único bloque libre del mayor orden posible.

---

## Safety

`BuddyAllocator` implementa `Send` manualmente (`unsafe impl Send`) porque no contiene referencias compartidas — los `NonNull` son punteros internos a la memoria del heap que el allocator gestiona exclusivamente.
