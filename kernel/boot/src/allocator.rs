//! A `GlobalAlloc` implementation deferring to UEFI memory boot services.

use core::alloc::{GlobalAlloc, Layout};

use crate::uefi;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `AllocatePool` returns 8-byte aligned regions.
        assert!(layout.align() <= 8);

        uefi::boot_services().allocate_pool(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        uefi::boot_services().free_pool(ptr)
    }
}
