use core::arch::asm;

use kstd::memory::{PA, VA};

pub const PAGE_SIZE: usize = 4 * (1 << 10);

/// Translate the given virtual address to a physical address.
pub fn va_to_pa(va: VA) -> PA {
    let par: u64;
    unsafe {
        asm!(
            "at s1e1r, {va}",
            "isb",
            "mrs {par}, par_el1",
            va = in(reg) u64::from(va),
            par = out(reg) par,
        );
    }

    if par & 1 != 0 {
        panic!("address translation failed (par={par:#x})");
    }

    PA::new(par & 0xffffffffffff000)
}
