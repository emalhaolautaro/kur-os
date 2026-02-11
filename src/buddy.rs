#![allow(unsafe_op_in_unsafe_fn)]

use core::ptr;

pub const PAGE_SIZE: usize = 4096;
pub const MIN_ORDER: usize = 12;
pub const MAX_ORDER: usize = 21; // 2 MB
const NUM_ORDERS: usize = MAX_ORDER - MIN_ORDER + 1;

#[repr(C)]
struct FreeBlock {
    next: Option<ptr::NonNull<FreeBlock>>,
}

pub struct BuddyAllocator {
    heap_start: usize,
    heap_size: usize,
    free_lists: [Option<ptr::NonNull<FreeBlock>>; NUM_ORDERS],
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        Self {
            heap_start: 0,
            heap_size: 0,
            free_lists: [None; NUM_ORDERS],
        }
    }

    pub fn start(&self) -> usize {
        self.heap_start
    }

    pub fn size(&self) -> usize {
        self.heap_size
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_memory(heap_start, heap_size);
    }

    pub unsafe fn add_memory(&mut self, start: usize, size: usize) {
        if self.heap_start == 0 {
            self.heap_start = start;
        }
        
        let mut current_start = start;
        let mut remaining_size = size;

        while remaining_size > 0 {
            let max_order = core::cmp::min(
                MAX_ORDER,
                self.size_to_order(remaining_size)
            );
            
            let mut order = max_order;
            while order >= MIN_ORDER {
                let size = 1 << order;
                if current_start % size == 0 {
                    break;
                }
                order -= 1;
            }
            
            if order < MIN_ORDER {
                 current_start += PAGE_SIZE;
                 remaining_size -= PAGE_SIZE;
                 continue;
            }

             let size = 1 << order;

             let block = current_start as *mut FreeBlock;
             (*block).next = None; 
             
             self.free_block(current_start, order);

             current_start += size;
             remaining_size -= size;
             self.heap_size += size;
        }
    }

    pub fn allocate(&mut self, size: usize) -> *mut u8 {
        let size = size.max(PAGE_SIZE);
        let order = self.size_to_order(size);

        if order > MAX_ORDER {
            return ptr::null_mut();
        }

        for current_order in order..=MAX_ORDER {
            let list_index = current_order - MIN_ORDER;

            if let Some(block) = self.free_lists[list_index] {
                unsafe {
                    self.free_lists[list_index] = (*block.as_ptr()).next;
                    self.split_block(block.as_ptr() as usize, current_order, order);
                    return block.as_ptr() as *mut u8;
                }
            }
        }

        ptr::null_mut()
    }

    pub unsafe fn deallocate(&mut self, ptr: *mut u8, size: usize) {
        let size = size.max(PAGE_SIZE);
        let order = self.size_to_order(size);
        self.free_block(ptr as usize, order);
    }

    unsafe fn split_block(&mut self, addr: usize, current_order: usize, target_order: usize) {
        let mut order = current_order;

        while order > target_order {
            order -= 1;
            let buddy_size = 1 << order;
            let buddy_addr = addr + buddy_size;

            let buddy = buddy_addr as *mut FreeBlock;
            let list_index = order - MIN_ORDER;
            (*buddy).next = self.free_lists[list_index];
            self.free_lists[list_index] = ptr::NonNull::new(buddy);
        }
    }

    unsafe fn free_block(&mut self, addr: usize, order: usize) {
        let mut current_addr = addr;
        let mut current_order = order;

        while current_order < MAX_ORDER {
            let block_size = 1 << current_order;
            let buddy_addr = self.buddy_address(current_addr, block_size);

            if buddy_addr < self.heap_start || buddy_addr >= self.heap_start + self.heap_size {
                break;
            }

            let list_index = current_order - MIN_ORDER;
            if !self.remove_from_free_list(buddy_addr, list_index) {
                break;
            }

            current_addr = current_addr.min(buddy_addr);
            current_order += 1;
        }

        let block = current_addr as *mut FreeBlock;
        let list_index = current_order - MIN_ORDER;
        (*block).next = self.free_lists[list_index];
        self.free_lists[list_index] = ptr::NonNull::new(block);
    }

    #[inline]
    fn buddy_address(&self, addr: usize, block_size: usize) -> usize {
        self.heap_start + ((addr - self.heap_start) ^ block_size)
    }

    unsafe fn remove_from_free_list(&mut self, addr: usize, list_index: usize) -> bool {
        let mut current = &mut self.free_lists[list_index];

        while let Some(block) = *current {
            if block.as_ptr() as usize == addr {
                *current = (*block.as_ptr()).next;
                return true;
            }
            current = &mut (*block.as_ptr()).next;
        }

        false
    }

    #[inline]
    fn size_to_order(&self, size: usize) -> usize {
        let size = size.next_power_of_two();
        size.trailing_zeros() as usize
    }

    #[inline]
    pub fn order_to_size(order: usize) -> usize {
        1 << order
    }
}

unsafe impl Send for BuddyAllocator {}
