use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};

use aarch64::memory::{PAGE_SIZE, VA};
use freelist::{ALIGN, FreeList, round_up_align};
use kstd::sync::Mutex;

use crate::memory::virt::{self, KHEAP_SIZE, KHEAP_START, PageNr};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeapAllocator = LockedHeapAllocator::new();

struct LockedHeapAllocator(Mutex<HeapAllocator>);

impl LockedHeapAllocator {
    const fn new() -> Self {
        Self(Mutex::new(HeapAllocator::new()))
    }
}

unsafe impl GlobalAlloc for LockedHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align() <= ALIGN);
        match self.0.lock().alloc(layout.size()) {
            Some(ptr) => ptr.as_ptr(),
            None => ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let ptr = NonNull::new(ptr).unwrap();
        unsafe { self.0.lock().free(ptr, layout.size()) };
    }
}

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

    fn alloc(&mut self, size: usize) -> Option<NonNull<u8>> {
        let size = round_up_align(size);

        match self.freelist.carve(size) {
            Some(ptr) => Some(ptr),
            None => match self.grow(size) {
                Ok(()) => self.freelist.carve(size),
                Err(()) => None,
            },
        }
    }

    /// # Safety
    ///
    /// The given block of memory must currently be allocated via this allocator and must have no
    /// other users.
    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize) {
        let size = round_up_align(size);
        unsafe { self.freelist.insert(ptr, size) };

        // TODO reclaim physical memory
    }

    fn grow(&mut self, size: usize) -> Result<(), ()> {
        let size = round_up_page(size);
        let new_break = self.heap_break + size;
        let kheap_limit = KHEAP_START + KHEAP_SIZE;

        if new_break > kheap_limit {
            return Err(());
        }

        let mut vpn = PageNr::from_va(self.heap_break);
        while vpn.va() < new_break {
            virt::map_data_page(vpn);
            vpn += 1;
        }

        let ptr = NonNull::new(self.heap_break.as_mut_ptr()).unwrap();
        unsafe { self.freelist.insert(ptr, size) };

        self.heap_break = new_break;

        Ok(())
    }
}

fn round_up_page(x: usize) -> usize {
    debug_assert!(PAGE_SIZE.is_power_of_two());
    let a = PAGE_SIZE - 1;
    (x + a) & !a
}
