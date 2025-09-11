use aarch64::memory::paging::PAGE_SIZE;
use aarch64::memory::PA;
use kstd::sync::Mutex;

use crate::memory::pa_to_va;

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

        let va = pa_to_va(pa);

        let next_pa = unsafe { va.as_mut_ptr::<PA>().read() };
        self.freelist = Some(next_pa);

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

        let va = pa_to_va(pa);

        // Insert the frame into the freelist.
        let next_pa = self.freelist.unwrap_or(PA::new(0));
        unsafe { va.as_mut_ptr::<PA>().write(next_pa) };

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
    unsafe { FRAME_ALLOCATOR.lock().free(pa) }
}

/// Free a range of page frames.
///
/// # Safety
///
/// `pa` must point to a range of `count` unused page frames.
pub(super) unsafe fn free_frames(mut pa: PA, count: usize) {
    for _ in 0..count {
        unsafe { free_frame(pa) };
        pa += PAGE_SIZE;
    }
}
