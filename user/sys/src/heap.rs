use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

use crate::sync::Mutex;

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator::new();

struct GlobalAllocator {
    alloc: Mutex<Allocator>,
}

impl GlobalAllocator {
    const fn new() -> Self {
        Self {
            alloc: Mutex::new(Allocator::new()),
        }
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut alloc = self.alloc.lock();
        let offset = alloc.start.align_offset(layout.align());
        let total_size = layout.size() + offset;
        if total_size > alloc.size {
            return ptr::null_mut();
        }

        let start = unsafe { alloc.start.add(offset) };
        let end = unsafe { start.add(layout.size()) };

        alloc.start = end;
        alloc.size -= total_size;

        start
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

struct Allocator {
    start: *mut u8,
    size: usize,
}

impl Allocator {
    const fn new() -> Self {
        Self {
            start: ptr::null_mut(),
            size: 0,
        }
    }
}

pub unsafe fn init(heap_start: *mut u8, heap_size: usize) {
    let mut alloc = GLOBAL_ALLOCATOR.alloc.lock();
    alloc.start = heap_start;
    alloc.size = heap_size;
}
