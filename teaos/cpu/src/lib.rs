#![no_std]

pub mod vmem;

use core::arch::asm;

pub fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
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
            "mrs {cnt}, cntvct_el0",
            "mrs {frq}, cntfrq_el0",
            cnt = out(reg) count,
            frq = out(reg) freq,
            options(nomem, preserves_flags, nostack),
        );
    }

    count * 1_000 / freq
}
