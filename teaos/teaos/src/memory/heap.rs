use core::alloc::{GlobalAlloc, Layout};
use core::mem;
use core::ptr::{self, NonNull};

use aarch64::memory::VA;
use kstd::sync::Mutex;

use crate::memory::phys;
use crate::memory::virt::{self, KHEAP_SIZE, KHEAP_START, PageNr};

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
        let new_break = self.heap_break + round_up_page(size);
        let kheap_limit = KHEAP_START + KHEAP_SIZE;

        if new_break >= kheap_limit {
            return Err(());
        }

        let mut vpn = PageNr::from_va(self.heap_break);
        while vpn.va() < new_break {
            let frame = phys::alloc();
            virt::map_ram(vpn, frame);
            vpn += 1;
        }

        let ptr = NonNull::new(self.heap_break.as_mut_ptr()).unwrap();
        self.freelist.insert(ptr, size);

        self.heap_break = new_break;

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
        let mut new_block_ptr = ptr.cast();

        // Find the insertion point, according to address order. Also keep track of the previous
        // block, we might need it for coalescing.
        let mut prev = None;
        let mut this = &mut self.head;
        while let Some(mut block_ptr) = *this {
            if block_ptr > new_block_ptr {
                break;
            }

            let block = unsafe { block_ptr.as_mut() };
            this = &mut block.next;
            prev = Some(block_ptr);
        }

        // Insert the new block.
        unsafe { new_block_ptr.write(FreeBlock { size, next: *this }) };
        *this = Some(new_block_ptr);

        // Coalesce with neighbors, if possible.
        unsafe { new_block_ptr.as_mut().try_coalesce() };
        if let Some(mut prev_ptr) = prev {
            unsafe { prev_ptr.as_mut().try_coalesce() };
        }
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

impl FreeBlock {
    /// Coalesce this block with the next one, if possible.
    fn try_coalesce(&mut self) {
        let Some(next_start) = self.next else {
            return;
        };

        let this_start = NonNull::from_mut(self);
        let this_end = unsafe { this_start.byte_add(self.size) };

        if this_end == next_start {
            let next = unsafe { next_start.as_ref() };
            self.size += next.size;
            self.next = next.next;
        }
    }
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
