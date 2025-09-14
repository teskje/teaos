use aarch64::memory::Frame;
use kstd::sync::Mutex;

use crate::memory::pa_to_va;

static FRAME_ALLOCATOR: Mutex<FrameAllocator> = Mutex::new(FrameAllocator::new());

/// An allocator for page frames.
struct FrameAllocator {
    freelist: Option<Frame>,
}

impl FrameAllocator {
    const fn new() -> Self {
        Self { freelist: None }
    }

    fn alloc(&mut self) -> Frame {
        let Some(frame) = self.freelist else {
            panic!("no free frames left to allocate");
        };

        let va = pa_to_va(frame.base());

        // Remove the first frame from the freelist.
        let next_frame = unsafe { va.as_mut_ptr::<Option<Frame>>().read() };
        self.freelist = next_frame;

        frame
    }

    /// # Safety
    ///
    /// `frame` must point to an unused page frame.
    unsafe fn free(&mut self, frame: Frame) {
        let va = pa_to_va(frame.base());

        // Insert the frame into the freelist.
        let next_frame = self.freelist;
        unsafe { va.as_mut_ptr::<Option<Frame>>().write(next_frame) };

        self.freelist = Some(frame);
    }
}

/// Allocate a page frame.
pub(super) fn alloc_frame() -> Frame {
    FRAME_ALLOCATOR.lock().alloc()
}

/// Free the given page frame.
///
/// # Safety
///
/// `frame` must point to an unused page frame.
pub(super) unsafe fn free_frame(frame: Frame) {
    unsafe { FRAME_ALLOCATOR.lock().free(frame) }
}

/// Free a range of page frames.
///
/// # Safety
///
/// `frame` must point to a range of `count` unused page frames.
pub(super) unsafe fn free_frames(start: Frame, count: usize) {
    let mut frame = start;
    for _ in 0..count {
        unsafe { free_frame(frame) };
        frame = frame.next_frame();
    }
}
