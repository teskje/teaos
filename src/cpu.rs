use core::arch::asm;

pub fn virtual_time() -> f64 {
    let count: u64;
    let freq: u64;
    unsafe {
        asm!(
            "mrs {cnt}, CNTVCT_EL0",
            "mrs {frq}, CNTFRQ_EL0",
            cnt = out(reg) count,
            frq = out(reg) freq,
        );
    }

    count as f64 / freq as f64
}
