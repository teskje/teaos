use core::arch::asm;
use core::sync::atomic::{compiler_fence, Ordering};

pub fn isb() {
    compiler_fence(Ordering::SeqCst);
    unsafe {
        asm!("isb", options(preserves_flags, nostack));
    }
    compiler_fence(Ordering::SeqCst);
}

pub fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}
