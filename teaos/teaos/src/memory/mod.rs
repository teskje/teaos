//! Memory management support.

mod paging;
mod phys;
mod virt;

use aarch64::memory::PA;
use aarch64::register::TTBR1_EL1;
use boot::info;
use phys::{alloc_frame, free_frames};

use crate::memory::paging::PageMap;
use crate::println;

pub use virt::{pa_to_va, KSTACK_END};

/// Initialize the memory subsystem.
///
/// Here we seed the frame allocator, take over the kernel translation tables, and initialize the
/// heap allocator, unlocking use of the `alloc` crate.
pub unsafe fn init(info: &info::Memory) {
    println!("initializing memory management");

    println!("  seeding frame allocator with unused blocks");
    for block in &info.blocks {
        if block.type_ == info::MemoryType::Unused {
            free_frames(block.start, block.pages);
        }
    }

    init_translation_tables();
}

/// Initialize kernel translation tables.
///
/// This takes over the kernel (TTBR1) translation tables from the boot loader by cloning them into
/// a new set of tables and then switching over to those. Doing so allows us to later free all boot
/// loader memory without having to make exceptions for the translation tables.
fn init_translation_tables() {
    println!("  initializing kernel translation tables");

    let ttbr = TTBR1_EL1::read();
    let boot_ttb = PA::new(ttbr.BADDR() << 1);
    let boot_map = PageMap::with_root(boot_ttb);

    let mut kernel_map = PageMap::new();
    kernel_map.clone_from(&boot_map);

    // SAFETY: New map contains all existing mappings.
    unsafe { kernel_map.load_ttbr1() };

    // TODO init global state
}
