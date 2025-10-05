//! Virtual memory management.

mod layout;
mod page_map;
mod page_table;

use core::ops::{Add, AddAssign};

use aarch64::instruction::{dsb_ishst, isb};
use aarch64::memory::paging::{Flags, load_ttbr1};
use aarch64::memory::{PA, PAGE_SHIFT, VA};
use kstd::sync::Mutex;

use crate::memory::phys::{self, FrameNr, FrameRef};

use self::layout::PHYSMAP_START;
use self::page_map::KernelPageMap;

pub use self::layout::*;

static VMM: Mutex<Option<VirtMemoryManager>> = Mutex::new(None);

/// A virtual page number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageNr(u64);

impl PageNr {
    pub fn from_va(va: VA) -> Self {
        assert!(va.is_page_aligned());
        Self(va.into_u64() >> PAGE_SHIFT)
    }

    pub fn va(&self) -> VA {
        VA::new(self.0 << PAGE_SHIFT)
    }
}

impl Add<u64> for PageNr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for PageNr {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

struct VirtMemoryManager {
    kernel_map: KernelPageMap,
}

impl VirtMemoryManager {
    fn map_data_page(&mut self, vpn: PageNr, frame: FrameRef) {
        let flags = Flags::default().privileged_execute_never(true);
        self.kernel_map.map_ram_page(vpn, frame, flags);

        // Wait for the new mapping to become visible.
        // Note that we don't need to TLBI here, since there wasn't a valid mapping for the VA before
        // (`PageMap::map_page` checks that).
        dsb_ishst();
        isb();
    }

    fn map_mmio_page(&mut self, vpn: PageNr, pfn: FrameNr) {
        let flags = Flags::default().privileged_execute_never(true);
        self.kernel_map.map_mmio_page(vpn, pfn, flags);

        // Wait for the new mapping to become visible.
        // Note that we don't need to TLBI here, since there wasn't a valid mapping for the VA before
        // (`PageMap::map_page` checks that).
        dsb_ishst();
        isb();
    }
}

pub fn pa_to_va(pa: PA) -> VA {
    PHYSMAP_START + u64::from(pa)
}

/// Initialize the virtual memory manager.
///
/// # Safety
///
/// The VMM must not have been initialized previously. In particular, the kernel page tables must
/// not be actively referenced by any code.
pub(super) unsafe fn init() {
    let mut vmm = VMM.lock();
    assert!(vmm.is_none(), "VMM already initialized");

    // SAFETY: No references to the kernel page tables exist.
    let kernel_map = unsafe { KernelPageMap::clone_from_ttbr1() };

    // SAFETY: New map contains all existing mappings.
    unsafe { load_ttbr1(kernel_map.base()) };

    *vmm = Some(VirtMemoryManager { kernel_map });
}

pub fn map_data_page(vpn: PageNr) {
    let frame = phys::alloc();

    let mut vmm = VMM.lock();
    vmm.as_mut()
        .expect("vmm initialized")
        .map_data_page(vpn, frame);
}

pub fn map_mmio_page(pfn: FrameNr) {
    let va = pa_to_va(pfn.pa());
    let vpn = PageNr::from_va(va);

    let mut vmm = VMM.lock();
    vmm.as_mut()
        .expect("vmm initialized")
        .map_mmio_page(vpn, pfn);
}
