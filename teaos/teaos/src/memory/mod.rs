//! Memory management support.

mod phys;
mod virt;

use aarch64::memory::paging::{TranslationTable, PAGE_SIZE};
use aarch64::memory::{PA, VA};
use aarch64::register::TTBR1_EL1;
use boot::info;
use phys::{alloc_frame, free_frames};
use virt::PHYS_START;

use crate::println;

pub unsafe fn init(info: &info::Memory) {
    println!("initializing memory management");

    println!("  seeding frame allocator with unused blocks");
    for block in &info.blocks {
        if block.type_ == info::MemoryType::Unused {
            free_frames(block.start, block.pages);
        }
    }

    init_translation_tables(info);

    println!("  reclaiming boot memory blocks");
    for block in &info.blocks {
        if block.type_ == info::MemoryType::Boot {
            free_frames(block.start, block.pages);
        }
    }
}

fn init_translation_tables(info: &info::Memory) {
    println!("  initializing kernel translation tables");

    let phys_offset = VA::new(0);

    let ttbr = TTBR1_EL1::read();
    let boot_ttb = PA::new(ttbr.BADDR() << 1);
    let boot_tt = TranslationTable::with_base(boot_ttb, phys_offset);

    let mut kernel_tt = TranslationTable::new(phys_offset, alloc_frame);
    kernel_tt.clone_from(&boot_tt, alloc_frame);

    for block in &info.blocks {
        let start_pa = block.start;
        let start_va = pa_to_va(start_pa);
        let size = block.pages * PAGE_SIZE;
        kernel_tt.map_region(start_va, start_pa, size, alloc_frame);
    }

    kernel_tt.load();
}

fn pa_to_va(pa: PA) -> VA {
    PHYS_START + u64::from(pa)
}
