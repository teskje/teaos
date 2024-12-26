use core::arch::asm;

pub fn virtual_time() -> f64 {
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

    count as f64 / freq as f64
}

pub fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}
