#![allow(unsafe_op_in_unsafe_fn)]

use core::ptr;
use crate::buddy::{BuddyAllocator, PAGE_SIZE};

const CACHE_SIZES: [usize; 9] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];
const NUM_CACHES: usize = CACHE_SIZES.len();
pub const MAX_SLAB_SIZE: usize = 2048;

#[repr(C)]
struct FreeObject {
    next: Option<ptr::NonNull<FreeObject>>,
}

struct Slab {
    next: Option<ptr::NonNull<Slab>>,
    free_list: Option<ptr::NonNull<FreeObject>>,
    free_count: usize,
    object_size: usize,
}

impl Slab {
    unsafe fn init(addr: usize, object_size: usize) -> *mut Slab {
        let slab = addr as *mut Slab;

        let header_size = core::mem::size_of::<Slab>();
        let data_start = addr + header_size;
        let data_start = (data_start + object_size - 1) & !(object_size - 1);

        let data_end = addr + PAGE_SIZE;
        let num_objects = (data_end - data_start) / object_size;

        let mut free_list: Option<ptr::NonNull<FreeObject>> = None;
        for i in (0..num_objects).rev() {
            let obj_addr = data_start + i * object_size;
            let obj = obj_addr as *mut FreeObject;
            (*obj).next = free_list;
            free_list = ptr::NonNull::new(obj);
        }

        (*slab).next = None;
        (*slab).free_list = free_list;
        (*slab).free_count = num_objects;
        (*slab).object_size = object_size;

        slab
    }

    unsafe fn allocate(&mut self) -> Option<*mut u8> {
        if let Some(obj) = self.free_list {
            self.free_list = (*obj.as_ptr()).next;
            self.free_count -= 1;
            Some(obj.as_ptr() as *mut u8)
        } else {
            None
        }
    }

    unsafe fn deallocate(&mut self, ptr: *mut u8) {
        let obj = ptr as *mut FreeObject;
        (*obj).next = self.free_list;
        self.free_list = ptr::NonNull::new(obj);
        self.free_count += 1;
    }
}

struct SlabCache {
    partial_slabs: Option<ptr::NonNull<Slab>>,
    full_slabs: Option<ptr::NonNull<Slab>>,
    object_size: usize,
}

impl SlabCache {
    const fn new(object_size: usize) -> Self {
        Self {
            partial_slabs: None,
            full_slabs: None,
            object_size,
        }
    }

    unsafe fn allocate(&mut self, buddy: &mut BuddyAllocator) -> *mut u8 {
        if let Some(slab) = self.partial_slabs {
            let slab_ptr = slab.as_ptr();
            if let Some(ptr) = (*slab_ptr).allocate() {
                if (*slab_ptr).free_count == 0 {
                    self.partial_slabs = (*slab_ptr).next;
                    (*slab_ptr).next = self.full_slabs;
                    self.full_slabs = Some(slab);
                }
                return ptr;
            }
        }

        let page = buddy.allocate(PAGE_SIZE);
        if page.is_null() {
            return ptr::null_mut();
        }

        let slab = Slab::init(page as usize, self.object_size);
        let ptr = (*slab).allocate().unwrap();

        (*slab).next = self.partial_slabs;
        self.partial_slabs = ptr::NonNull::new(slab);

        ptr
    }

    unsafe fn deallocate(&mut self, ptr: *mut u8) {
        let slab_addr = (ptr as usize) & !(PAGE_SIZE - 1);
        let slab = slab_addr as *mut Slab;

        let was_full = (*slab).free_count == 0;

        (*slab).deallocate(ptr);

        if was_full {
            Self::remove_slab_from_list(&mut self.full_slabs, slab);
            (*slab).next = self.partial_slabs;
            self.partial_slabs = ptr::NonNull::new(slab);
        }
    }

    unsafe fn remove_slab_from_list(list: &mut Option<ptr::NonNull<Slab>>, target: *mut Slab) {
        let mut current = list;
        while let Some(slab) = *current {
            if slab.as_ptr() == target {
                *current = (*slab.as_ptr()).next;
                return;
            }
            current = &mut (*slab.as_ptr()).next;
        }
    }
}

pub struct SlabAllocator {
    caches: [SlabCache; NUM_CACHES],
    buddy: BuddyAllocator,
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            caches: [
                SlabCache::new(8),
                SlabCache::new(16),
                SlabCache::new(32),
                SlabCache::new(64),
                SlabCache::new(128),
                SlabCache::new(256),
                SlabCache::new(512),
                SlabCache::new(1024),
                SlabCache::new(2048),
            ],
            buddy: BuddyAllocator::new(),
        }
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.buddy.init(heap_start, heap_size);
    }

    pub fn allocate(&mut self, size: usize, align: usize) -> *mut u8 {
        let effective_size = size.max(align);

        if effective_size <= MAX_SLAB_SIZE {
            if let Some(cache_index) = self.find_cache_index(effective_size) {
                unsafe { self.caches[cache_index].allocate(&mut self.buddy) }
            } else {
                ptr::null_mut()
            }
        } else {
            self.buddy.allocate(effective_size)
        }
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, size: usize, align: usize) {
        let effective_size = size.max(align);

        if effective_size <= MAX_SLAB_SIZE {
            if let Some(cache_index) = self.find_cache_index(effective_size) {
                self.caches[cache_index].deallocate(ptr);
            }
        } else {
            self.buddy.deallocate(ptr, effective_size);
        }
    }

    fn find_cache_index(&self, size: usize) -> Option<usize> {
        for (i, &cache_size) in CACHE_SIZES.iter().enumerate() {
            if size <= cache_size {
                return Some(i);
            }
        }
        None
    }
}

unsafe impl Send for SlabAllocator {}
