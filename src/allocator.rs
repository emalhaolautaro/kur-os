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
            let mut allocator = self.inner.lock();
            let mut ptr = allocator.allocate(layout.size(), layout.align());
            
            if ptr.is_null() {
                let size = layout.size().max(layout.align());
                let block_size = size.next_power_of_two().max(crate::buddy::PAGE_SIZE);
                
                let current_end = allocator.start() + allocator.size();
                
                let start_page = Page::containing_address(VirtAddr::new(current_end as u64));
                let end_addr = current_end + block_size;
                let end_page = Page::containing_address(VirtAddr::new(end_addr as u64 - 1));
                
                let page_range = Page::range_inclusive(start_page, end_page);
                
                let mut mapping_success = true;
                for page in page_range {
                    if crate::memory::map_page(page).is_err() {
                        mapping_success = false;
                        break;
                    }
                }
                
                if mapping_success {
                    allocator.add_memory(current_end, block_size);
                    ptr = allocator.allocate(layout.size(), layout.align());
                }
            }
            
            ptr
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
        mapper::MapToError, Page, Size4KiB,
    },
    VirtAddr,
};

pub fn init_heap() -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        crate::memory::map_page(page)?;
    }

    unsafe {
        ALLOCATOR.init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}