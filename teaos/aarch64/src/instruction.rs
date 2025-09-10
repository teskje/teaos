use core::arch::asm;

use crate::memory::VA;

#[inline(always)]
pub fn dsb_ish() {
    unsafe {
        asm!("dsb ish", options(preserves_flags, nostack));
    }
}

#[inline(always)]
pub fn dsb_ishst() {
    unsafe {
        asm!("dsb ishst", options(preserves_flags, nostack));
    }
}

#[inline(always)]
pub fn isb() {
    unsafe {
        asm!("isb", options(preserves_flags, nostack));
    }
}

#[inline(always)]
pub fn tlbi_vae1is(va: VA) {
    unsafe {
        asm!(
            "tlbi vae1is, {x}",
            x = in(reg) va.into_u64() >> 12,
            options(preserves_flags, nostack),
        );
    }
}

#[inline(always)]
pub fn tlbi_vmalle1is() {
    unsafe {
        asm!("tlbi vmalle1is", options(preserves_flags, nostack));
    }
}

#[inline(always)]
pub fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}
