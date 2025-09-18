use crate::instruction::{dsb_ish, dsb_ishst, isb, tlbi_vae1is, tlbi_vmalle1is};
use crate::register::{MAIR_EL1, TCR_EL1, TTBR1_EL1};

use super::{PA, PAGE_SIZE, VA};

pub struct MairIndexes {
    pub device: u8,
    pub normal: u8,
}

impl MairIndexes {
    pub fn read() -> Self {
        let mut device = None;
        let mut normal = None;

        let mut check = |idx, attr| {
            if attr == 0x00 {
                device = Some(idx);
            } else if attr == 0xff {
                normal = Some(idx);
            }
        };

        let mair = MAIR_EL1::read();
        check(0, mair.ATTR0());
        check(1, mair.ATTR1());
        check(2, mair.ATTR2());
        check(3, mair.ATTR3());
        check(4, mair.ATTR4());
        check(5, mair.ATTR5());
        check(6, mair.ATTR6());
        check(7, mair.ATTR7());

        Self {
            device: device.expect("missing device attr"),
            normal: normal.expect("missing normal attr"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shareability {
    None = 0b00,
    Inner = 0b11,
    Outer = 0b10,
}

/// Load a page map into TTBR1.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and all
/// existing mappings still required by existing threads are also present in the new mappings.
pub unsafe fn load_ttbr1(ttb: PA) {
    let mut tcr = TCR_EL1::read();
    tcr.set_T1SZ(16);
    tcr.set_EPD1(0);
    tcr.set_IRGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_ORGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_SH1(0b11); // (inner shareable)
    tcr.set_TG1(0b10); // (4 KiB)

    // Make previous translation table writes visible.
    dsb_ishst();

    unsafe {
        TTBR1_EL1::write(ttb);
        TCR_EL1::write(tcr);
    }

    // Make sure all subsequent instructions use the new translation tables.
    isb();

    // Invalidate all EL1 TLB entries.
    tlbi_vmalle1is();

    // Wait for TLBI to complete and refetch.
    dsb_ish();
    isb();
}

/// Disable address translation using TTBR0.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and no
/// TTBR0 mappings are still required by existing threads.
pub unsafe fn disable_ttbr0() {
    let mut tcr = TCR_EL1::read();
    tcr.set_EPD0(1);

    unsafe { TCR_EL1::write(tcr) };

    // Make sure all subsequent instructions observe the change.
    isb();

    // Invalidate all EL1 TLB entries.
    tlbi_vmalle1is();

    // Wait for TLBI to complete and refetch.
    dsb_ish();
    isb();
}

pub fn tlb_invalidate(mut va: VA, size: usize) {
    assert!(va.is_page_aligned(), "unaligned: {va:#}");

    let end = va + size;

    // Make previous translation table writes visible.
    dsb_ishst();

    // Invalidate all pages in range.
    while va < end {
        tlbi_vae1is(va);
        va += PAGE_SIZE;
    }

    // Wait for TLBIs to complete and refetch.
    dsb_ish();
    isb();
}
