#![cfg_attr(not(test), no_std)]

pub mod log;

mod memory;
mod uart;

use boot::info::{self, BootInfo};

use crate::uart::Uart;

/// # Safety
///
/// The provided `bootinfo` must contain correct memory addresses.
pub unsafe fn kernel(bootinfo: &BootInfo) -> ! {
    init_logging(&bootinfo.uart);
    println!("enterned kernel");

    print_bootinfo(bootinfo);

    // Seed the page allocator with the unused pages.
    for block in &bootinfo.memory.blocks {
        if block.type_ == info::MemoryType::Unused {
            memory::free_pages(block.start, block.pages);
        }
    }

    let pa = memory::alloc_page();
    println!("allocated a page: {pa:#}");

    cpu::halt();
}

unsafe fn init_logging(uart_info: &info::Uart) {
    let uart = match uart_info {
        info::Uart::Pl011 { base } => Uart::pl011(base.as_mut_ptr()),
        info::Uart::Uart16550 { base } => Uart::uart16550(base.as_mut_ptr()),
    };

    log::init(uart);
}

fn print_bootinfo(bootinfo: &BootInfo) {
    let BootInfo {
        memory,
        uart,
        acpi_rsdp,
    } = bootinfo;

    println!("bootinfo.memory:");
    println!("     start        pages    type");
    println!("  ------------------------------");
    for block in &memory.blocks {
        println!(
            "  {:#012}  {:8}  {}",
            block.start, block.pages, block.type_
        );
    }
    println!("bootinfo.uart: {uart:?}");
    println!("bootinfo.acpi_rsdp: {acpi_rsdp:#}");
}
