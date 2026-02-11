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
pub unsafe fn init(physical_memory_offset: VirtAddr, memory_map: &'static MemoryMap) {
    let level_4_table = active_level_4_table(physical_memory_offset);
    let mapper = OffsetPageTable::new(level_4_table, physical_memory_offset);
    let frame_allocator = BootInfoFrameAllocator::init(memory_map);

    *MAPPER.lock() = Some(mapper);
    *FRAME_ALLOCATOR.lock() = Some(frame_allocator);
}
```

1. Lee el registro `CR3` para obtener la dirección física de la tabla de páginas de nivel 4.
2. Inicializa las estructuras `OffsetPageTable` y `BootInfoFrameAllocator`.
3. Almacena estas instancias en variables estáticas globales protegidas por `Mutex` (`MAPPER` y `FRAME_ALLOCATOR`), permitiendo acceso seguro desde cualquier parte del kernel (crucial para el allocator).

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

---

## Mapeo Dinámico (`map_page`)

Para soportar la **expansión del heap**, el módulo de memoria expone una función pública segura:

```rust
pub fn map_page(page: Page) -> Result<(), MapToError<Size4KiB>> {
    // ... obtiene locks a MAPPER y FRAME_ALLOCATOR ...
    if mapper.translate_page(page).is_ok() {
        return Ok(()); // Idempotente: si ya existe, éxito.
    }
    let frame = frame_allocator.allocate_frame()?;
    unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    Ok(())
}
```

Esto permite al allocator solicitar memoria física arbitraria para nuevas páginas virtuales bajo demanda.

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

Se mantiene `translate_addr` como ejercicio educativo y fallback, delegando a `translate_addr_inner`.

---

## Frame Allocator

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

---

## Ejemplo de mapeo (`create_example_mapping`)

Función de prueba que mapea una página arbitraria al frame VGA (`0xb8000`), usando ahora los recursos globales:

```rust
pub fn create_example_mapping(page: Page) {
    let mut mapper_lock = MAPPER.lock();
    let mut frame_allocator_lock = FRAME_ALLOCATOR.lock();
    
    if let (Some(mapper), Some(frame_allocator)) = (mapper_lock.as_mut(), frame_allocator_lock.as_mut()) {
        let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
        let flags = Flags::PRESENT | Flags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator).expect("failed").flush();
        }
    }
}
```
