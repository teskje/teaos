use aarch64::memory::{PA, PAGE_SIZE, VA, va_to_pa};

use crate::memory::phys::FrameNr;
use crate::memory::{pa_to_va, virt};

#[derive(Debug)]
pub struct MmioPage {
    base: VA,
}

impl MmioPage {
    /// # Safety
    ///
    /// `offset` must point to a readable MMIO register of type `T`.
    pub unsafe fn read<T: Copy>(&self, offset: usize) -> T {
        debug_assert!(offset < PAGE_SIZE);

        let va = self.base + offset;
        unsafe { va.as_ptr::<T>().read_volatile() }
    }

    /// # Safety
    ///
    /// `offset` must point to a writable MMIO register of type `T`.
    pub unsafe fn write<T: Copy>(&mut self, offset: usize, val: T) {
        debug_assert!(offset < PAGE_SIZE);

        let va = self.base + offset;
        unsafe { va.as_mut_ptr::<T>().write_volatile(val) }
    }
}

/// Claim the given MMIO page.
///
/// # Safety
///
/// `pa` must reference an MMIO page frame.
/// There must be no concurrent owner of that MMIO page.
pub unsafe fn claim_page(pa: PA) -> MmioPage {
    assert!(pa.is_page_aligned());

    let va = pa_to_va(pa);

    if va_to_pa(va).is_none() {
        let pfn = FrameNr::from_pa(pa);
        virt::map_mmio_page(pfn);
    }

    MmioPage { base: va }
}
