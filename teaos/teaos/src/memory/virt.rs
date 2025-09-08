//! Virtual memory management.
//!
//! The kernel lives in high virtual memory:
//!
//!  0xffff000000000000 - 0xffff00003fffffff    kernel code + data
//!  0xffff000040000000 - 0xffff000040003fff    stack (16 KiB)
//!  0xffff000080000000 - 0xffff0000ffffffff    heap (2 GiB)
//!  0xffff100000000000 - 0xffffffffffffffff    physical memory mapping (240 TiB)

use core::arch::global_asm;
use core::ffi::c_void;

use aarch64::memory::{PA, VA};

pub const KERNEL_START: VA = VA::new(0xffff000000000000);
pub const KSTACK_START: VA = VA::new(0xffff000040000000);
pub const KSTACK_SIZE: usize = 16 << 10;
pub const PHYS_START: VA = VA::new(0xffff100000000000);

global_asm!(
    r#"
    .globl kernel_start, kstack_start, kstack_end, phys_start
    kernel_start = {kernel_start}
    kstack_start = {kstack_start}
    phys_start   = {phys_start}

    .section .kstack, "aw", %nobits
    .space {kstack_size}
    .globl _kstack_end
    _kstack_end:
    "#,
    kernel_start = const KERNEL_START.into_u64(),
    kstack_start = const KSTACK_START.into_u64(),
    kstack_size = const KSTACK_SIZE,
    phys_start = const PHYS_START.into_u64(),
);

extern "C" {
    #[link_name = "_kstack_end"]
    pub static KSTACK_END: c_void;
}

pub fn pa_to_va(pa: PA) -> VA {
    PHYS_START + u64::from(pa)
}
