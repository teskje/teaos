use crate::instruction::{dsb_ish, dsb_ishst, isb, tlbi_vae1is, tlbi_vmalle1is};
use crate::register::{MAIR_EL1, TCR_EL1, TTBR0_EL1, TTBR1_EL1};

use super::{PA, PAGE_SIZE, VA};

#[derive(Clone, Copy, Debug, Default)]
pub struct Flags(u64);

impl Flags {
    pub fn attr_idx(self, x: u8) -> Self {
        self.set(x, 2, 0b111)
    }

    pub fn access_permissions(self, x: AccessPermissions) -> Self {
        self.set(x, 6, 0b11)
    }

    pub fn shareability(self, x: Shareability) -> Self {
        self.set(x, 8, 0b11)
    }

    pub fn access_flag(self, x: bool) -> Self {
        self.set(x, 10, 0b1)
    }

    pub fn privileged_execute_never(self, x: bool) -> Self {
        self.set(x, 53, 0b1)
    }

    pub fn unprivileged_execute_never(self, x: bool) -> Self {
        self.set(x, 54, 0b1)
    }

    fn set<X: Into<u64>>(mut self, x: X, shift: u64, mask: u64) -> Self {
        self.0 &= !(mask << shift);
        self.0 |= x.into() << shift;
        self
    }
}

impl From<Flags> for u64 {
    fn from(flags: Flags) -> Self {
        flags.0
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AccessPermissions {
    PrivRW = 0b00,
    UnprivRW = 0b01,
    PrivRO = 0b10,
    UnprivRO = 0b11,
}

impl From<AccessPermissions> for u64 {
    fn from(value: AccessPermissions) -> Self {
        value as u64
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shareability {
    None = 0b00,
    Inner = 0b11,
    Outer = 0b10,
}

impl From<Shareability> for u64 {
    fn from(value: Shareability) -> Self {
        value as u64
    }
}

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

/// Load a page map into TTBR1.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and all
/// existing mappings still required by existing threads are also present in the new mappings.
pub unsafe fn load_ttbr1(baddr: PA) {
    let mut tcr = TCR_EL1::read();
    tcr.set_T1SZ(16);
    tcr.set_EPD1(0);
    tcr.set_IRGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_ORGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_SH1(0b11); // (inner shareable)
    tcr.set_TG1(0b10); // (4 KiB)

    let mut ttbr1 = TTBR1_EL1::default();
    ttbr1.set_BADDR(u64::from(baddr) >> 1);

    // Make previous translation table writes visible.
    dsb_ishst();

    unsafe {
        TTBR1_EL1::write(ttbr1);
        TCR_EL1::write(tcr);
    }

    // Make sure all subsequent instructions use the new translation tables.
    isb();
}

/// Load a page map into TTBR0.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and all
/// existing mappings still required by existing threads are also present in the new mappings.
pub unsafe fn load_ttbr0(baddr: PA, asid: u8) {
    let mut tcr = TCR_EL1::read();
    tcr.set_T0SZ(16);
    tcr.set_EPD0(0);
    tcr.set_IRGN0(0b01); // (normal memory, WBWA cacheable)
    tcr.set_ORGN0(0b01); // (normal memory, WBWA cacheable)
    tcr.set_SH0(0b11); // (inner shareable)
    tcr.set_TG0(0b00); // (4 KiB)
    tcr.set_A1(0b0);

    let mut ttbr0 = TTBR0_EL1::default();
    ttbr0.set_BADDR(u64::from(baddr) >> 1);
    ttbr0.set_ASID(asid.into());

    // Make previous translation table writes visible.
    dsb_ishst();

    unsafe {
        TTBR0_EL1::write(ttbr0);
        TCR_EL1::write(tcr);
    }

    // Make sure all subsequent instructions use the new translation tables.
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

    tlb_invalidate_all();
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

pub fn tlb_invalidate_all() {
    // Make previous translation table writes visible.
    dsb_ishst();

    // Invalidate all EL1 TLB entries.
    tlbi_vmalle1is();

    // Wait for TLBI to complete and refetch.
    dsb_ish();
    isb();
}
