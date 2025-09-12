use core::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{self, NonNull};

use aarch64::memory::VA;
use aarch64::memory::paging::{MemoryClass, PAGE_SIZE};
use kstd::sync::Mutex;

use crate::memory::alloc_frame;
use crate::memory::paging::map_page;
use crate::memory::virt::{KHEAP_SIZE, KHEAP_START};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeapAllocator = LockedHeapAllocator::new();

struct HeapAllocator {
    freelist: FreeList,
    heap_break: VA,
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
            freelist: FreeList::new(),
            heap_break: KHEAP_START,
        }
    }

    fn alloc(&mut self, size: usize) -> *mut u8 {
        let size = round_up_align(size);

        let ptr = match self.freelist.carve(size) {
            Some(ptr) => ptr,
            None => match self.grow(size) {
                Ok(()) => self.freelist.carve(size).unwrap(),
                Err(()) => return ptr::null_mut(),
            },
        };

        ptr.as_ptr()
    }

    fn free(&mut self, ptr: *mut u8, size: usize) {
        let size = round_up_align(size);

        let ptr = NonNull::new(ptr).unwrap();
        self.freelist.insert(ptr, size);

        // TODO reclaim physical memory
    }

    fn grow(&mut self, size: usize) -> Result<(), ()> {
        let size = round_up_page(size);
        let new_break = self.heap_break + round_up_page(size);
        let kheap_limit = KHEAP_START + KHEAP_SIZE;

        if new_break >= kheap_limit {
            return Err(());
        }

        let mut va = self.heap_break;
        while va < new_break {
            let pa = alloc_frame();
            map_page(va, pa, MemoryClass::Normal);
            va += PAGE_SIZE;
        }

        let ptr = NonNull::new(self.heap_break.as_mut_ptr()).unwrap();
        self.freelist.insert(ptr, size);

        self.heap_break = va;

        Ok(())
    }
}

/// A linked list of free blocks of heap memory.
///
/// Invariants:
///  * blocks are sorted in address order
///  * (TODO) blocks are maximally coalesced
struct FreeList {
    head: Option<NonNull<FreeBlock>>,
}

impl FreeList {
    const fn new() -> Self {
        Self { head: None }
    }

    fn carve(&mut self, size: usize) -> Option<NonNull<u8>> {
        let mut head = &mut self.head;

        while let Some(mut block_ptr) = *head {
            let block = unsafe { block_ptr.as_mut() };

            if block.size == size {
                *head = block.next;
                return Some(block_ptr.cast());
            }

            if block.size > size {
                let rest = block.size - size;
                debug_assert!(rest >= mem::size_of::<FreeBlock>());

                unsafe {
                    let new_block_ptr = block_ptr.byte_add(size);
                    new_block_ptr.write(FreeBlock {
                        size: rest,
                        next: block.next,
                    });
                    *head = Some(new_block_ptr);
                }

                return Some(block_ptr.cast());
            }

            head = &mut block.next;
        }

        None
    }

    fn insert(&mut self, ptr: NonNull<u8>, size: usize) {
        debug_assert!(size >= mem::size_of::<FreeBlock>());
        let new_block_ptr = ptr.cast();

        let mut head = &mut self.head;
        while let Some(mut block_ptr) = *head {
            if block_ptr > new_block_ptr {
                break;
            }

            let block = unsafe { block_ptr.as_mut() };
            head = &mut block.next;
        }

        unsafe {
            new_block_ptr.write(FreeBlock { size, next: *head });
        }
        *head = Some(new_block_ptr);

        // TODO coalesce
    }
}

fn round_up_align(x: usize) -> usize {
    (x + 15) & !15
}

fn round_up_page(x: usize) -> usize {
    (x + 4095) & !4095
}

struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>,
}

struct LockedHeapAllocator(Mutex<HeapAllocator>);

impl LockedHeapAllocator {
    const fn new() -> Self {
        Self(Mutex::new(HeapAllocator::new()))
    }
}

unsafe impl GlobalAlloc for LockedHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `HeapAllocator` returns 16-byte aligned allocations
        assert!(layout.align() <= 16);
        self.0.lock().alloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.0.lock().free(ptr, layout.size());
    }
}
