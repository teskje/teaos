//! Memory management support.

mod heap;
mod paging;
mod phys;
mod virt;

use crate::log;
use crate::memory::phys::{alloc_frame, free_frames};

pub use virt::{KSTACK_END, pa_to_va};

/// Initialize the memory subsystem.
///
/// Here we seed the frame allocator, take over the kernel translation tables, and initialize the
/// heap allocator, unlocking use of the `alloc` crate.
pub unsafe fn init(info: &boot_info::Memory) {
    log!("initializing memory management");

    log!("  seeding frame allocator with unused blocks");
    for block in &info.blocks {
        if block.type_ == boot_info::MemoryType::Unused {
            unsafe { free_frames(block.start, block.pages) };
        }
    }

    log!("  initializing kernel paging");
    paging::init();
}
