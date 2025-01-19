use aarch64::memory::{PA, PAGE_SIZE};
use kstd::sync::Mutex;

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

/// An allocator for page frames.
struct FrameAllocator {
    freelist: Option<PA>,
}

impl FrameAllocator {
    const fn new() -> Self {
        Self { freelist: None }
    }

    fn alloc(&mut self) -> PA {
        let Some(pa) = self.freelist else {
            panic!("no free frames left to allocate");
        };

        let next_pa = unsafe { pa.as_mut_ptr::<PA>().read() };
        self.freelist = Some(next_pa);

        unsafe {
            fill_frame(pa, 0x00);
        }

        pa
    }

    /// # Safety
    ///
    /// `pa` must point to an unused page frame.
    unsafe fn free(&mut self, pa: PA) {
        assert!(
            pa.is_aligned_to(PAGE_SIZE),
            "pa {pa:#} not aligned to page size"
        );

        // Fill the frame with garbage, to help catch UAF bugs.
        fill_frame(pa, 0xab);

        // Insert the frame into the freelist.
        let next_pa = self.freelist.unwrap_or(PA::new(0));
        pa.as_mut_ptr::<PA>().write(next_pa);

        self.freelist = Some(pa);
    }
}

/// Allocate a page frame.
pub(super) fn alloc_frame() -> PA {
    FRAME_ALLOCATOR.lock().alloc()
}

/// Free the page frame at the given `pa`.
///
/// # Safety
///
/// `pa` must point to an unused page frame.
pub(super) unsafe fn free_frame(pa: PA) {
    FRAME_ALLOCATOR.lock().free(pa);
}

/// Free a range of page frames.
///
/// # Safety
///
/// `pa` must point to a range of `count` unused page frames.
pub(super) unsafe fn free_frames(mut pa: PA, count: usize) {
    for _ in 0..count {
        free_frame(pa);
        pa += PAGE_SIZE;
    }
}

/// # Safety
///
/// `pa` must point to an unused page frame.
unsafe fn fill_frame(pa: PA, fill: u8) {
    let frame = &mut *pa.as_mut_ptr::<[u8; PAGE_SIZE]>();
    frame.fill(fill);
}
