pub mod paging;

mod address;

pub use self::address::{PA, VA};

pub const PAGE_SIZE: usize = 4 << 10;
pub const PAGE_MAP_LEVELS: u64 = 3;

/// A physical memory page frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame(PA);

impl Frame {
    pub const fn new(base: PA) -> Self {
        assert!(base.is_page_aligned());

        Self(base)
    }

    pub const fn base(self) -> PA {
        self.0
    }

    pub fn next_frame(self) -> Frame {
        Self(self.0 + PAGE_SIZE)
    }
}

/// A virtual memory page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page(VA);

impl Page {
    pub const fn new(base: VA) -> Self {
        assert!(base.is_page_aligned());

        Self(base)
    }

    pub const fn base(self) -> VA {
        self.0
    }

    pub fn next_page(self) -> Page {
        Self(self.0 + PAGE_SIZE)
    }

    pub const fn as_ptr(self) -> *const [u8; PAGE_SIZE] {
        self.0.as_ptr()
    }

    pub const fn as_mut_ptr(self) -> *mut [u8; PAGE_SIZE] {
        self.0.as_mut_ptr()
    }
}

/// Trait for page frame allocators.
pub trait FrameAlloc {
    /// Allocate a new page frame.
    fn alloc_frame() -> Frame;
}

/// Trait for PA-to-VA address mappers.
pub trait AddrMapper {
    /// Map the given PA to a VA that can be used to access that physical memory location.
    fn pa_to_va(pa: PA) -> VA;

    /// Map the given frame to a page.
    fn frame_to_page(frame: Frame) -> Page {
        let va = Self::pa_to_va(frame.base());
        Page::new(va)
    }
}
