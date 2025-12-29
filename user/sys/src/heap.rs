use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{self, NonNull};

use freelist::{ALIGN, FreeList, round_up_align};

use crate::sync::Mutex;

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
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
            freelist: FreeList::new(),
        }
    }

    fn alloc(&mut self, size: usize) -> Option<NonNull<u8>> {
        let size = round_up_align(size);
        self.freelist.carve(size)
    }

    /// # Safety
    ///
    /// The given block of memory must currently be allocated via this allocator and must have no
    /// other users.
    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize) {
        let size = round_up_align(size);
        unsafe { self.freelist.insert(ptr, size) };
    }
}

pub unsafe fn init(heap_start: NonNull<u8>, heap_size: usize) {
    let mut alloc = HEAP_ALLOCATOR.0.lock();
    unsafe { alloc.freelist.insert(heap_start, heap_size) };
}
