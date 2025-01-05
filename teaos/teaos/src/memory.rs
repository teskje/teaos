use kstd::memory::PA;
use kstd::sync::Mutex;

use cpu::vmem::PAGE_SIZE;

extern "C" {
    pub static __KERNEL_START: u8;
    pub static __KERNEL_END: u8;

    pub static __STACK_START: u8;
    pub static __STACK_END: u8;

    pub static __HEAP_START: u8;
    pub static __HEAP_END: u8;

    pub static __LINEAR_REGION_START: u8;
}

static PAGE_ALLOCATOR: Mutex<PageAllocator> = Mutex::new(PageAllocator::new());

/// An allocator for physical memory pages.
struct PageAllocator {
    freelist: Option<PA>,
}

impl PageAllocator {
    const fn new() -> Self {
        Self { freelist: None }
    }

    fn alloc(&mut self) -> PA {
        let Some(pa) = self.freelist else {
            panic!("no free pages left to allocate");
        };

        let next_pa = unsafe { pa.as_mut_ptr::<PA>().read() };
        self.freelist = Some(next_pa);

        unsafe { fill_page(pa, 0x00); }

        pa
    }

    /// # Safety
    ///
    /// `pa` must point to an unused page.
    unsafe fn free(&mut self, pa: PA) {
        assert!(
            pa.is_aligned_to(PAGE_SIZE),
            "pa {pa:#} not aligned to page size"
        );

        // Fill the page with garbage, to help catch UAF bugs.
        fill_page(pa, 0xab);

        // Insert the page into the freelist.
        let next_pa = self.freelist.unwrap_or(PA::new(0));
        pa.as_mut_ptr::<PA>().write(next_pa);

        self.freelist = Some(pa);
    }
}

/// Allocate a page.
pub fn alloc_page() -> PA {
    PAGE_ALLOCATOR.lock().alloc()
}

/// Free the page at the given `pa`.
///
/// # Safety
///
/// `pa` must point to an unused page.
pub unsafe fn free_page(pa: PA) {
    PAGE_ALLOCATOR.lock().free(pa);
}

/// Free a range of pages.
///
/// # Safety
///
/// `pa` must point to a range of `count` unused pages.
pub unsafe fn free_pages(mut pa: PA, count: usize) {
    for _ in 0..count {
        free_page(pa);
        pa += PAGE_SIZE;
    }
}

/// # Safety
///
/// `pa` must point to an unused page.
unsafe fn fill_page(pa: PA, fill: u8) {
    let page = &mut *pa.as_mut_ptr::<[u8; PAGE_SIZE]>();
    page.fill(fill);
}
