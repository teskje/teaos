//! Memory management support.

mod phys;
mod virt;

use aarch64::memory::paging::TranslationTable;
use aarch64::memory::{PA, VA};
use aarch64::register::TTBR1_EL1;
use boot::info;
use phys::{alloc_frame, free_frames};

use crate::println;

pub use virt::{pa_to_va, PHYS_START, KSTACK_END};

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
    let boot_tt = TranslationTable::with_base(boot_ttb, PHYS_START);

    let mut kernel_tt = TranslationTable::new(PHYS_START, alloc_frame);
    kernel_tt.clone_from(&boot_tt, alloc_frame);
    kernel_tt.load();
}

pub fn map_page(va: VA, pa: PA) {
    let ttbr1 = TTBR1_EL1::read();
    let base = PA::new(ttbr1.BADDR());
    let mut kernel_tt = TranslationTable::with_base(base, PHYS_START);
    kernel_tt.map_page(va, pa, alloc_frame);
}
