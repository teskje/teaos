//! Virtual memory management.
//!
//! The kernel lives in high virtual memory:
//!
//!  0xffff000000000000 - 0xffff00003fffffff    kernel code + data
//!  0xffff000040000000 - 0xffff000040003fff    stack (16 KiB)
//!  0xffff000080000000 - 0xffff0000ffffffff    heap (2 GiB)
//!  0xffff100000000000 - 0xffffffffffffffff    physical memory mapping (240 TiB)

use aarch64::memory::VA;

pub(super) const PHYS_START: VA = VA::new(0xffff100000000000);
