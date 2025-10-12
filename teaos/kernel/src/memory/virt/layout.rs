//! Kernel virtual memory layout.
//!
//! The kernel lives in high virtual memory:
//!
//!  0xffff000000000000 - 0xffff0000ffffffff    kernel code + data
//!  0xffff000100000000 - 0xffff000100003fff    stack (16 KiB)
//!  0xffff000200000000 - 0xffff0002ffffffff    heap (4 GiB)
//!  0xffff000300000000 - 0xffff0003ffffffff    userimg (4 GiB)
//!  0xffff100000000000 - 0xffffffffffffffff    physmap (240 TiB)

use core::arch::global_asm;
use core::ffi::c_void;

use aarch64::memory::VA;

pub const KERNEL_START: VA = VA::new(0xffff_0000_0000_0000);
pub const KSTACK_START: VA = VA::new(0xffff_0001_0000_0000);
pub const KSTACK_SIZE: usize = 16 << 10;
pub const KHEAP_START: VA = VA::new(0xffff_0002_0000_0000);
pub const KHEAP_SIZE: usize = 4 << 30;
pub const USERIMG_START: VA = VA::new(0xffff_0003_0000_0000);
pub const USERIMG_SIZE: usize = 4 << 30;
pub const PHYSMAP_START: VA = VA::new(0xffff_1000_0000_0000);

global_asm!(
    r#"
    .globl kernel_start, kstack_start, kstack_end, userimg_start, physmap_start
    kernel_start  = {kernel_start}
    kstack_start  = {kstack_start}
    userimg_start = {userimg_start}
    physmap_start = {physmap_start}

    .section .kstack, "aw", %nobits
    .space {kstack_size}
    .globl _kstack_end
    _kstack_end:
    "#,
    kernel_start = const KERNEL_START.into_u64(),
    kstack_start = const KSTACK_START.into_u64(),
    kstack_size = const KSTACK_SIZE,
    userimg_start = const USERIMG_START.into_u64(),
    physmap_start = const PHYSMAP_START.into_u64(),
);

unsafe extern "C" {
    #[link_name = "_kstack_end"]
    pub static KSTACK_END: c_void;
}
