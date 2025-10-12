use kstd::sync::Mutex;

use super::{FrameNr, pa_to_va};

static ALLOC: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

/// A physical page frame allocator.
struct FrameAllocator {
    freelist: Option<FrameNr>,
}

impl FrameAllocator {
    pub(super) const fn new() -> Self {
        Self { freelist: None }
    }

    fn alloc(&mut self) -> FrameNr {
        let Some(pfn) = self.freelist else {
            panic!("no free frames available");
        };

        let va = pa_to_va(pfn.pa());

        // Pop the first frame from the freelist.
        //
        // SAFETY: Reading what was previously written in `Self::free`. Frame was just retrieved
        // from the list of free frames, so no other readers or writers exist.
        let next_pfn = unsafe { va.as_mut_ptr::<Option<FrameNr>>().read() };
        self.freelist = next_pfn;

        pfn
    }

    /// # Safety
    ///
    /// `pfn` must identify an unused page frame.
    unsafe fn free(&mut self, pfn: FrameNr) {
        let va = pa_to_va(pfn.pa());

        // Insert the frame into the freelist.
        let next_frame = self.freelist;
        // SAFETY: Destination is page-aligned and points to a physical memory page. Frame is
        // unused, so no other readers or writers exist.
        unsafe { va.as_mut_ptr::<Option<FrameNr>>().write(next_frame) };

        self.freelist = Some(pfn);
    }
}

/// Allocate a page frame.
pub(super) fn alloc_frame() -> FrameNr {
    ALLOC.lock().alloc()
}

/// Free the given page frame.
///
/// # Safety
///
/// `pfn` must identify an unused page frame.
pub(super) unsafe fn free_frame(pfn: FrameNr) {
    unsafe { ALLOC.lock().free(pfn) }
}

