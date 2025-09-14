//! Memory management support.

mod heap;
mod paging;
mod phys;
mod virt;

use crate::log;

use aarch64::memory::Frame;
use aarch64::memory::paging::disable_ttbr0;
use boot_info::MemoryType;

use self::phys::free_frames;

pub use self::paging::map_page;
pub use self::virt::{KSTACK_END, pa_to_va};

/// Initialize the memory subsystem.
///
/// This initializes both physical and virtual memory management, unlocking the use of the `alloc`
/// crate. It also takes over all boot memory by removing the TTBR0 mappings and claiming all
/// loader memory for the frame allocator.
pub unsafe fn init(info: boot_info::Memory<'_>) {
    log!("initializing memory management");

    log!("  seeding frame allocator with unused blocks");
    for block in info.blocks {
        if block.type_ == MemoryType::Unused {
            let start = Frame::new(block.start);
            unsafe { free_frames(start, block.pages) };
        }
    }

    log!("  initializing kernel paging");
    paging::init();

    // Taking over the boot memory will make the bootinfo invalid, so copy what we still need and
    // then drop it.
    let memory_blocks = info.blocks.to_vec();
    drop(info);

    log!("  disabling boot page tables");
    // SAFETY: Not using any ttbr0 mappings anymore.
    unsafe { disable_ttbr0() };

    log!("  claiming boot memory for frame allocator");
    for block in memory_blocks {
        if block.type_ == MemoryType::Boot {
            let start = Frame::new(block.start);
            unsafe { free_frames(start, block.pages) };
        }
    }
}
