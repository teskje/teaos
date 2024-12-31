use core::alloc::{GlobalAlloc, Layout};

use crate::sync::Mutex;
use crate::uefi;

#[global_allocator]
static ALLOCATOR: Mutex<Allocator> = Mutex::new(Allocator::new());

pub fn init(boot_services: uefi::BootServices) {
    ALLOCATOR.lock().boot_services = Some(boot_services);
}

pub fn uninit() {
    ALLOCATOR.lock().boot_services = None;
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

unsafe impl GlobalAlloc for Mutex<Allocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let self_ = self.lock();
        let Some(boot_services) = &self_.boot_services else {
            panic!("allocator not initialized");
        };

        // `AllocatePool` returns 8-byte aligned regions.
        assert!(layout.align() <= 8);

        boot_services.allocate_pool(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let self_ = self.lock();
        let Some(boot_services) = &self_.boot_services else {
            panic!("allocator not initialized");
        };

        boot_services.free_pool(ptr)
    }
}
