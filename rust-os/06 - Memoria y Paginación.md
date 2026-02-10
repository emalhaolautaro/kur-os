# Memoria y Paginación

> **Archivo:** `src/memory.rs`
> **Propósito:** Inicializar page tables, traducir direcciones virtuales a físicas, y proveer un frame allocator basado en el memory map del bootloader.

---

## Modelo de memoria

El bootloader v0.9 mapea **toda la memoria física** al espacio virtual del kernel con un offset fijo (`physical_memory_offset`). Esto significa:

```
dirección_virtual = dirección_física + physical_memory_offset
```

Esto simplifica enormemente el acceso a page tables, ya que cualquier frame físico se puede leer/escribir directamente a través de su dirección virtual.

---

## Inicialización (`memory::init`)

```rust
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}
```

1. Lee el registro `CR3` para obtener la dirección física de la tabla de páginas de nivel 4
2. Convierte a dirección virtual usando el offset
3. Crea un `OffsetPageTable` — abstracción del crate `x86_64` que maneja la traducción de 4 niveles

### `active_level_4_table`

```rust
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}
```

> **Unsafe por dos razones:**
> 1. El caller debe garantizar que el offset es correcto
> 2. Solo debe llamarse una vez para evitar crear dos `&mut` al mismo page table

---

## Paginación en x86_64 — 4 niveles

```
Dirección Virtual (48 bits usados):
┌─────────┬─────────┬─────────┬─────────┬──────────────┐
│ P4 (9b) │ P3 (9b) │ P2 (9b) │ P1 (9b) │ Offset (12b) │
└─────────┴─────────┴─────────┴─────────┴──────────────┘

CR3 → Page Table L4
       └─ entrada[P4] → Page Table L3
                         └─ entrada[P3] → Page Table L2
                                          └─ entrada[P2] → Page Table L1
                                                           └─ entrada[P1] → Frame Físico
                                                                            + Offset = Dirección Física
```

Cada tabla tiene 512 entradas (9 bits de índice × 4 niveles = 36 bits + 12 bits de offset = 48 bits de espacio virtual).

---

## Traducción manual (`translate_addr`)

Además del `OffsetPageTable` del crate `x86_64`, el módulo implementa una traducción manual de 4 niveles como ejercicio educativo:

```rust
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame; // desde CR3

    for &index in &table_indexes {
        let table = /* convertir frame físico a referencia virtual */;
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameNotPresent) => return None,
            Err(HugeFrame) => panic!("páginas grandes no soportadas"),
        };
    }

    Some(frame.start_address() + addr.page_offset())
}
```

Recorre los 4 niveles de la tabla de páginas, resolviendo cada índice hasta llegar al frame físico final. No soporta huge pages (2MB/1GB).

> **Decisión de diseño:** La función unsafe `translate_addr` delega a `translate_addr_inner` (función segura) para limitar el alcance del bloque unsafe.

---

## Frame Allocator

### `EmptyFrameAllocator`

Implementación stub que siempre retorna `None`. Se usaba antes de tener el allocator real.

### `BootInfoFrameAllocator`

Allocator real que distribuye frames físicos del memory map provisto por el bootloader.

```rust
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,  // índice del siguiente frame a entregar
}
```

#### Método `usable_frames()`

```
MemoryMap → filtrar regiones USABLE → expandir en frames de 4KB → PhysFrame
```

1. Filtra las regiones marcadas como `Usable` por el bootloader
2. Expande cada región en direcciones de frame (step_by 4096)
3. Convierte cada dirección a un `PhysFrame`

#### Limitación actual

El método `allocate_frame()` usa `nth(self.next)` que recrea el iterador completo cada vez. Esto es **O(n)** donde n es el número de frames ya asignados.

> **Posible mejora futura:** Cachear el estado del iterador o usar un bitmap para tracking de frames disponibles.

---

## Ejemplo de mapeo (`create_example_mapping`)

Función de prueba que mapea una página arbitraria al frame VGA (`0xb8000`):

```rust
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;
    mapper.map_to(page, frame, flags, frame_allocator).flush();
}
```

Útil para verificar que el sistema de paginación funciona correctamente.
