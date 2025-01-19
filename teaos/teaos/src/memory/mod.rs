mod physical;

use boot::info;
use physical::free_frames;

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
}
