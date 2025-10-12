#![no_std]

use core::arch::asm;

pub fn sys_print(s: &str) {
    let ptr = s.as_ptr();
    let len = s.len();

    unsafe {
        asm!(
            "svc #0",
            in("x0") ptr,
            in("x1") len,
        )
    }
}
