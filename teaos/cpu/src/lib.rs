#![no_std]

pub mod vmem;

use core::arch::asm;

pub fn wfe() {
    unsafe {
        asm!("WFE", options(nomem, preserves_flags, nostack));
    }
}

/// Halt the CPU indefinitely.
pub fn halt() -> ! {
    loop {
        wfe();
    }
}

/// Return the CPU uptime, in milliseconds.
pub fn uptime() -> u64 {
    let count: u64;
    let freq: u64;
    unsafe {
        asm!(
            "MRS {cnt}, CNTVCT_EL0",
            "MRS {frq}, CNTFRQ_EL0",
            cnt = out(reg) count,
            frq = out(reg) freq,
            options(nomem, preserves_flags, nostack),
        );
    }

    count * 1_000 / freq
}
