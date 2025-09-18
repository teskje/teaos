use aarch64::memory::paging::{MairIndexes, Shareability};
use aarch64::memory::{PA, VA};
use aarch64::register::TTBR1_EL1;

use crate::memory::phys::{self, FrameNr, FrameRef};

use super::PageNr;
use super::page_table::{PageDesc, PageTable, PageTableRef};

/// A virtual memory page map.
pub(super) struct PageMap {
    level0: PageTable<0>,
    mair_idx: MairIndexes,
}

impl PageMap {
    pub fn new() -> Self {
        Self {
            level0: PageTable::new(),
            mair_idx: MairIndexes::read(),
        }
    }

    pub fn base(&self) -> PA {
        self.level0.base()
    }

    pub fn map_ram(&mut self, vpn: PageNr, frame: FrameRef) {
        let mut desc = PageDesc::new(frame.pa());

        desc.set_access_flag();
        desc.set_attr_idx(self.mair_idx.normal);
        desc.set_shareability(Shareability::Inner);

        frame.inc_map();
        // SAFETY: `inc_map` called above.
        unsafe { self.insert(vpn, desc) }
    }

    /// # Safety
    ///
    /// The caller must ensure that map counting is handled correctly for the mapped frame, either
    /// by calling [`FrameRef::inc_map`] or by ensuring that the page is never unmapped again. Note
    /// that `PageMap`'s `Drop` implementation unmaps all mapped pages.
    unsafe fn insert(&mut self, vpn: PageNr, desc: PageDesc) {
        let l0 = &mut self.level0;
        let mut l1 = l0.get_or_insert(vpn);
        let mut l2 = l1.get_or_insert(vpn);
        let mut l3 = l2.get_or_insert(vpn);

        // We disallow overwriting valid entries, forcing callers to follow proper
        // break-before-make procedure.
        assert!(l3.get(vpn).is_none(), "page {vpn:?} already mapped");

        l3.set(vpn, desc);
    }
}

impl Drop for PageMap {
    fn drop(&mut self) {
        let start_vpn = PageNr::from_va(VA::new(0));
        self.level0.walk(start_vpn, |vpn, desc| {
            let base = desc.output_addr();
            let pfn = FrameNr::from_pa(base);
            let frame = phys::get_alloc_frame(pfn).unwrap_or_else(|| {
                panic!("mapping for unallocated frame: {vpn:?} -> {pfn:?}");
            });

            // SAFETY: Page descriptors are only inserted through `PageMap::insert`, which requires
            // that `inc_map` was called before the `PageMap` gets dropped.
            unsafe { frame.dec_map() };
        });
    }
}

/// A `PageMap` for the kernel address space.
pub(super) struct KernelPageMap(PageMap);

impl KernelPageMap {
    pub fn base(&self) -> PA {
        self.0.base()
    }

    /// # Safety
    ///
    /// The page map under TTBR1 must not be modified concurrently.
    pub unsafe fn clone_from_ttbr1() -> Self {
        let ttbr1 = TTBR1_EL1::read();
        let base = PA::new(ttbr1.BADDR() << 1);
        // SAFETY: Page tables are not modified concurrently.
        let pt = unsafe { PageTableRef::<0>::new(base) };

        let mut map = PageMap::new();
        let start_vpn = PageNr::from_va(VA::new(0));
        pt.walk(start_vpn, |vpn, desc| {
            // SAFETY: Page is never unmapped again.
            unsafe { map.insert(vpn, desc) }
        });

        Self(map)
    }

    pub fn map_ram(&mut self, vpn: PageNr, frame: FrameRef) {
        self.0.map_ram(vpn, frame);
    }
}

impl Drop for KernelPageMap {
    fn drop(&mut self) {
        panic!("kernel page map must never be dropped");
    }
}
