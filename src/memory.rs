use x86_64::{
    // VirtAddr vive en la raíz de la crate
    VirtAddr,
    PhysAddr,
    structures::paging::{
        Page, PhysFrame, Mapper, Size4KiB, FrameAllocator, 
        OffsetPageTable, PageTable // PageTable vive aquí adentro
    }
};

use bootloader::bootinfo::MemoryMap;

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

/// Un FrameAllocator que devuelve marcos utilizables del mapa de memoria del bootloader.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Crea un FrameAllocator a partir del mapa de memoria pasado.
    ///
    /// Esta función es insegura porque el llamador debe garantizar que el mapa de memoria pasado
    /// sea válido. El principal requisito es que todos los marcos que están marcados
    /// como `USABLE` en él estén realmente sin usar.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// Devuelve un iterador sobre los marcos utilizables del mapa de memoria.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        use bootloader::bootinfo::MemoryRegionType;
        // Obtener regiones utilizables del mapa de memoria
        let regions = self.memory_map.iter();
        let usable_regions = regions
            .filter(|r| r.region_type == MemoryRegionType::Usable);
        
        // Mapear cada región a su rango de direcciones
        let addr_ranges = usable_regions
            .map(|r| r.range.start_addr()..r.range.end_addr());
        
        // Transformar a un iterador de direcciones de inicio de marco
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        
        // Crear objetos `PhysFrame` a partir de las direcciones de inicio
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

/// Inicializa una nueva OffsetPageTable.
///
/// Esta función es insegura porque el llamador debe garantizar que la
/// memoria física completa esté mapeada en memoria virtual en el pasado
/// `physical_memory_offset`. Además, esta función debe ser solo llamada una vez
/// para evitar aliasing de referencias `&mut` (lo que es comportamiento indefinido).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Devuelve una referencia mutable a la tabla de nivel 4 activa.
///
/// Esta función es insegura porque el llamador debe garantizar que la
/// memoria física completa esté mapeada en memoria virtual en el pasado
/// `physical_memory_offset`. Además, esta función solo debe ser llamada una vez
/// para evitar aliasing de referencias `&mut` (lo que es comportamiento indefinido).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// Traduce la dirección virtual dada a la dirección física mapeada, o
/// `None` si la dirección no está mapeada.
///
/// Esta función es insegura porque el llamador debe garantizar que la
/// memoria física completa esté mapeada en memoria virtual en el pasado
/// `physical_memory_offset`.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    translate_addr_inner(addr, physical_memory_offset)
}

/// Función privada que es llamada por `translate_addr`.
///
/// Esta función es segura para limitar el alcance de `unsafe` porque Rust trata
/// el cuerpo completo de las funciones inseguras como un bloque inseguro. Esta función debe
/// solo ser alcanzable a través de `unsafe fn` desde fuera de este módulo.
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr)
    -> Option<PhysAddr>
{
    use x86_64::structures::paging::page_table::FrameError;
    use x86_64::registers::control::Cr3;

    // leer el marco de nivel 4 activo desde el registro CR3
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()
    ];
    let mut frame = level_4_table_frame;

    // recorrer la tabla de páginas de múltiples niveles
    for &index in &table_indexes {
        // convertir el marco en una referencia a la tabla de páginas
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe {&*table_ptr};

        // leer la entrada de la tabla de páginas y actualizar `frame`
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("páginas grandes no soportadas"),
        };
    }

    // calcular la dirección física sumando el desplazamiento de página
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

/// Crea un mapeo de ejemplo para la página dada al marco `0xb8000`.
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        // FIXME: esto no es seguro, lo hacemos solo para pruebas
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to falló").flush();
}