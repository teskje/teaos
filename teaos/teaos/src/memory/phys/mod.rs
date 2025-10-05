//! Physical memory management.

mod alloc;

use core::mem;
use core::num::NonZeroU8;
use core::sync::atomic::{self, AtomicU32, Ordering};

use aarch64::memory::{PA, PAGE_SHIFT, PAGE_SIZE};
use kstd::sync::Mutex;

use self::alloc::{alloc_frame, free_frame};
use super::pa_to_va;

static PMM: Mutex<PhysMemoryManager> = Mutex::new(PhysMemoryManager::new());

/// A physical page frame number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FrameNr(u64);

impl FrameNr {
    pub fn from_pa(pa: PA) -> Self {
        assert!(pa.is_page_aligned());
        Self(pa.into_u64() >> PAGE_SHIFT)
    }

    pub fn pa(&self) -> PA {
        PA::new(self.0 << PAGE_SHIFT)
    }
}

/// Metadata tracked about an allocated page frame.
struct Frame {
    pfn: FrameNr,
    refcount: AtomicU32,
    // A niche to ensure an `Option<Frame>` remains 16 bytes in size.
    _niche: NonZeroU8,
}

impl Frame {
    fn new(pfn: FrameNr) -> Self {
        Self {
            pfn,
            refcount: AtomicU32::new(0),
            _niche: NonZeroU8::new(1).unwrap(),
        }
    }

    fn inc_ref(&self) {
        self.refcount.fetch_add(1, Ordering::Release);
    }

    fn dec_ref(&self) {
        self.refcount.fetch_sub(1, Ordering::Release);
    }
}

pub struct FrameRef {
    frame: *const Frame,
}

impl FrameRef {
    /// # Safety
    ///
    /// The memory location of the provided `Frame` must be stable for as long as its `refcount` is
    /// greater than zero.
    unsafe fn new(frame: &Frame) -> Self {
        frame.inc_ref();

        Self { frame }
    }

    fn frame(&self) -> &Frame {
        // SAFETY: As long as `self` is alive the frame has at least one counted reference
        // preventing it from being dropped.
        unsafe { &*self.frame }
    }

    pub fn pa(&self) -> PA {
        self.frame().pfn.pa()
    }

    /// Obtain a temporary view of the frame contents.
    ///
    /// # Panics
    ///
    /// Panics if there exist other references to the same frame.
    pub fn with_contents(&mut self, f: impl FnOnce(&mut [u8; PAGE_SIZE])) {
        // Take the PMM lock to ensure no new references can be created while we view the frame
        // contents.
        let _pmm = PMM.lock();

        assert_eq!(self.frame().refcount.load(Ordering::Acquire), 1);

        let va = pa_to_va(self.pa());
        let ptr = va.as_mut_ptr();

        // SAFETY: No other references to this frame exist.
        let buf = unsafe { &mut *ptr };

        f(buf)
    }

    /// Increment the map count.
    pub fn inc_map(&self) {
        self.frame().inc_ref();
    }

    /// Decrement the map count.
    ///
    /// # Safety
    ///
    /// This method must only be called once for every call to `inc_map`.
    pub unsafe fn dec_map(&self) {
        self.frame().dec_ref();
    }
}

impl Drop for FrameRef {
    fn drop(&mut self) {
        let frame = self.frame();

        // Take the PMM lock before decrementing the refcount. If we reduce the count to zero, this
        // ensures that nobody can acquire a new `FrameRef` before the frame was freed.
        let mut pmm = PMM.lock();

        // The ordering/fencing here is cargo-culted from `Arc::drop`. See the code comments there
        // for the rationale.
        if frame.refcount.fetch_sub(1, Ordering::Release) == 1 {
            atomic::fence(Ordering::Acquire);

            pmm.free(frame.pfn);
        }
    }
}

/// Physical memory manager.
struct PhysMemoryManager {
    frames: FrameMap,
}

impl PhysMemoryManager {
    const fn new() -> Self {
        Self {
            frames: FrameMap::new(),
        }
    }

    fn alloc(&mut self) -> FrameRef {
        let pfn = alloc_frame();
        let frame = Frame::new(pfn);
        let old = self.frames.insert(pfn, frame);
        assert!(old.is_none());

        self.get_alloc_frame(pfn).expect("inserted above")
    }

    /// # Panics
    ///
    /// Panics if the given `pfn` identifies a frame that wasn't previously allocated, or a frame
    /// that still has live references.
    fn free(&mut self, pfn: FrameNr) {
        match self.frames.remove(pfn) {
            Some(frame) => assert_eq!(frame.refcount.load(Ordering::Acquire), 0),
            None => panic!("attempt to free unallocated frame: {pfn:?}"),
        }

        // SAFETY: Frame is known to have zero references.
        unsafe { free_frame(pfn) };
    }

    fn get_alloc_frame(&self, pfn: FrameNr) -> Option<FrameRef> {
        self.frames.get(pfn).map(|frame| {
            // SAFETY: Frames in the `FrameMap` are never moved and only freed once their refcount
            // drops to zero.
            unsafe { FrameRef::new(frame) }
        })
    }
}

type L1 = [Option<&'static mut L2>; 512];
type L2 = [Option<&'static mut L3>; 512];
type L3 = [Option<&'static mut L4>; 512];
type L4 = [Option<Frame>; 256];

/// A map of allocated frames, keyed by [`FrameNr`].
///
/// The map is structured like a page table, consisting of a five-level tree of page-sized nodes.
/// To perform a lookup, a PFN is split into five level indexes:
///
///          +----------------------------------------------------+
///   PFN:   | 35 | 34 ... 26 | 25 ... 17 | 16 ...  8 |  7 ...  0 |
///          +----------------------------------------------------+
///   level:   0        1           2           3           4
///
/// Level pages are lazily allocated on `insert`, but never freed. In exchange the implementation
/// gets to use references instead of raw pointers internally, so we're trading memory efficiency
/// for convenience.
///
/// The addresses of [`Frame`]s stored in the map are guaranteed to be stable, and the
/// implementation of [`FrameRef`] relies on this fact.
struct FrameMap {
    level0: [Option<&'static mut L1>; 2],
}

impl FrameMap {
    const fn new() -> Self {
        Self {
            level0: [None, None],
        }
    }

    fn get(&self, pfn: FrameNr) -> Option<&Frame> {
        let l1 = self.level0[Self::level_idx(pfn, 0)].as_ref()?;
        let l2 = l1[Self::level_idx(pfn, 1)].as_ref()?;
        let l3 = l2[Self::level_idx(pfn, 2)].as_ref()?;
        let l4 = l3[Self::level_idx(pfn, 3)].as_ref()?;
        l4[Self::level_idx(pfn, 4)].as_ref()
    }

    fn insert(&mut self, pfn: FrameNr, frame: Frame) -> Option<Frame> {
        fn alloc_level<T, const N: usize>() -> &'static mut [Option<T>; N] {
            assert_eq!(mem::size_of::<[Option<T>; N]>(), PAGE_SIZE);

            let pfn = alloc_frame();
            let va = pa_to_va(pfn.pa());
            let ptr = va.as_mut_ptr::<[Option<T>; N]>();

            // SAFETY: `ptr` points to an unused frame the same size as `[Option<T>; N]`
            unsafe {
                ptr.write([const { None }; N]);
                &mut *ptr
            }
        }

        let l1 = self.level0[Self::level_idx(pfn, 0)].get_or_insert_with(alloc_level);
        let l2 = l1[Self::level_idx(pfn, 1)].get_or_insert_with(alloc_level);
        let l3 = l2[Self::level_idx(pfn, 2)].get_or_insert_with(alloc_level);
        let l4 = l3[Self::level_idx(pfn, 3)].get_or_insert_with(alloc_level);
        l4[Self::level_idx(pfn, 4)].replace(frame)
    }

    fn remove(&mut self, pfn: FrameNr) -> Option<Frame> {
        let l1 = self.level0[Self::level_idx(pfn, 0)].as_mut()?;
        let l2 = l1[Self::level_idx(pfn, 1)].as_mut()?;
        let l3 = l2[Self::level_idx(pfn, 2)].as_mut()?;
        let l4 = l3[Self::level_idx(pfn, 3)].as_mut()?;
        l4[Self::level_idx(pfn, 4)].take()
    }

    fn level_idx(pfn: FrameNr, level: u64) -> usize {
        let bits = pfn.0 as usize;
        match level {
            0 => (bits >> 35) & 0x1,
            1 => (bits >> 26) & 0x1ff,
            2 => (bits >> 17) & 0x1ff,
            3 => (bits >> 8) & 0x1ff,
            4 => bits & 0xff,
            5.. => unreachable!(),
        }
    }
}

/// Allocate a page frame.
pub(super) fn alloc() -> FrameRef {
    PMM.lock().alloc()
}

/// Allocate a page frame filled with zeroes.
pub(super) fn alloc_zero() -> FrameRef {
    let mut frame = alloc();
    frame.with_contents(|buf| buf.fill(0));
    frame
}

/// Return a reference to an allocated frame.
pub(super) fn get_alloc_frame(pfn: FrameNr) -> Option<FrameRef> {
    PMM.lock().get_alloc_frame(pfn)
}

/// Seed the physical memory allocator with a chunk of memory.
///
/// # Safety
///
/// The provided range must describe a valid RAM range. All memory in this range must be unused.
pub(super) unsafe fn seed(start: PA, pages: usize) {
    let mut pa = start;
    for _ in 0..pages {
        let pfn = FrameNr::from_pa(pa);
        // SAFETY: Frame known to be unused.
        unsafe { free_frame(pfn) };
        pa += PAGE_SIZE;
    }
}
