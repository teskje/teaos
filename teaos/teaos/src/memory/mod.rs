mod phys;

use aarch64::memory::paging::TranslationTable;
use aarch64::memory::{PA, VA};
use aarch64::register::TTBR1_EL1;
use boot::info;
use phys::{alloc_frame, free_frames};

use crate::println;

extern "C" {
    static __KERNEL_START: u8;
    static __KERNEL_END: u8;

    static __STACK_START: u8;
    static __STACK_END: u8;

    static __HEAP_START: u8;
    static __HEAP_END: u8;

    static __LINEAR_REGION_START: u8;
}

pub unsafe fn init(info: &info::Memory) {
    println!("initializing memory management");

    println!("seeding frame allocator with unused blocks");
    for block in &info.blocks {
        if block.type_ == info::MemoryType::Unused {
            free_frames(block.start, block.pages);
        }
    }

    init_translation_tables();
    map_physical_memory(info);

    println!("reclaiming boot memory blocks");
    for block in &info.blocks {
        if block.type_ == info::MemoryType::Boot {
            free_frames(block.start, block.pages);
        }
    }
}

fn init_translation_tables() {
    println!("initializing kernel translation tables");

    let phys_offset = VA::new(0);

    let ttbr = TTBR1_EL1::read();
    let boot_ttb = PA::new(ttbr.BADDR() << 1);
    let boot_tt = TranslationTable::with_base(boot_ttb, phys_offset);

    let mut kernel_tt = TranslationTable::new(phys_offset, alloc_frame);
    kernel_tt.clone_from(&boot_tt, alloc_frame);
    kernel_tt.load();
}

fn map_physical_memory(info: &info::Memory) {}
