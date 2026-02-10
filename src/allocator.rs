#![allow(unsafe_op_in_unsafe_fn)]

use alloc::alloc::{GlobalAlloc, Layout};
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::slab::SlabAllocator;

pub use crate::buddy::PAGE_SIZE;

pub const HEAP_SIZE: usize = 128 * 1024;
pub const HEAP_START: usize = 0x_4444_4442_0000;

pub struct LockedSlabAllocator {
    inner: Mutex<SlabAllocator>,
}

impl LockedSlabAllocator {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(SlabAllocator::new()),
        }
    }

    pub unsafe fn init(&self, heap_start: usize, heap_size: usize) {
        self.inner.lock().init(heap_start, heap_size);
    }
}

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

#[global_allocator]
static ALLOCATOR: LockedSlabAllocator = LockedSlabAllocator::new();

use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush()
        };
    }

    assert!(
        HEAP_START % HEAP_SIZE == 0,
        "HEAP_START ({:#x}) debe estar alineado a HEAP_SIZE ({:#x})",
        HEAP_START, HEAP_SIZE
    );
    assert!(
        HEAP_SIZE.is_power_of_two(),
        "HEAP_SIZE ({:#x}) debe ser potencia de 2",
        HEAP_SIZE
    );
    assert!(
        HEAP_SIZE >= crate::buddy::PAGE_SIZE,
        "HEAP_SIZE ({:#x}) debe ser >= PAGE_SIZE ({:#x})",
        HEAP_SIZE, crate::buddy::PAGE_SIZE
    );

    unsafe {
        ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}