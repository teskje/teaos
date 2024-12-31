use core::alloc::{GlobalAlloc, Layout};

use crate::uefi;

#[global_allocator]
static mut ALLOCATOR: Allocator = Allocator::new();

pub fn init(boot_services: uefi::BootServices) {
    unsafe {
        let allocator = &raw mut ALLOCATOR;
        (*allocator).boot_services = Some(boot_services);
    }
}

pub fn uninit() {
    unsafe {
        let allocator = &raw mut ALLOCATOR;
        (*allocator).boot_services = None;
    }
}

struct Allocator {
    boot_services: Option<uefi::BootServices>,
}

impl Allocator {
    const fn new() -> Self {
        Self {
            boot_services: None,
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(boot_services) = &self.boot_services else {
            panic!("allocator not initialized");
        };

        // `AllocatePool` returns 8-byte aligned regions.
        assert!(layout.align() <= 8);

        boot_services.allocate_pool(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let Some(boot_services) = &self.boot_services else {
            panic!("allocator not initialized");
        };

        boot_services.free_pool(ptr)
    }
}
