//! Memory management support.

pub mod mmio;
pub mod phys;
pub mod virt;

mod heap;

use crate::log;

use aarch64::memory::paging::disable_ttbr0;
use boot_info::MemoryType;

pub use self::virt::pa_to_va;

/// Initialize the memory subsystem.
///
/// This initializes both physical and virtual memory management, unlocking the use of the `alloc`
/// crate. It also takes over all boot memory by removing the TTBR0 mappings and claiming all
/// loader memory for the frame allocator.
///
/// # Safety
///
/// The memory subsystem must not have been initialized previously.
/// The given boot info must accurately describe the system physical memory.
pub unsafe fn init(info: boot_info::Memory<'_>) {
    log!("initializing memory management");

    log!("  seeding PMM with unused blocks");
    for block in info.blocks {
        if block.type_ == MemoryType::Unused {
            // SAFETY: Block is unused, according to the boot info.
            unsafe { phys::seed(block.start, block.pages) };
        }
    }

    log!("  initializing VMM");
    // SAFETY: No references to TTBR1 page tables exist.
    unsafe { virt::init() };

    // Taking over the boot memory will make the bootinfo invalid, so copy what we still need and
    // then drop it.
    let memory_blocks = info.blocks.to_vec();
    drop(info);

    log!("  disabling boot page tables");
    // SAFETY: Not using any TTBR0 mappings anymore.
    unsafe { disable_ttbr0() };

    log!("  claiming boot memory");
    for block in memory_blocks {
        if block.type_ == MemoryType::Boot {
            // SAFETY: Block hasn't been given to the PMM before and is now unused since we've
            // taken over all boot memory.
            unsafe { phys::seed(block.start, block.pages) };
        }
    }
}
