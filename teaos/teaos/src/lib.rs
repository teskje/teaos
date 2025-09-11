//! The TeaOS kernel.

#![cfg_attr(not(test), no_std)]

pub mod log;

mod exception;
mod memory;
mod uart;

use core::arch::naked_asm;

use boot::info::{self, BootInfo};

use crate::memory::{pa_to_va, KSTACK_END};
use crate::uart::Uart;

/// The kernel entry point.
///
/// This is a tiny assembly stub that runs before `kernel_main` to set up the kernel stack.
///
/// # Safety
///
/// The provided `bootinfo` must contain correct memory addresses.
#[unsafe(naked)]
pub unsafe extern "C" fn start(bootinfo: &BootInfo) -> ! {
    naked_asm!(
        r#"
        ldr x9, ={kstack_end}
        mov sp, x9

        b {main}
        "#,
        kstack_end = sym KSTACK_END,
        main = sym kernel_main,
    )
}

/// The kernel main function.
///
/// # Safety
///
/// The provided `bootinfo` must contain correct memory addresses.
unsafe extern "C" fn kernel_main(bootinfo: &BootInfo) -> ! {
    init_logging(&bootinfo.uart);
    log!("enterned kernel");

    print_bootinfo(bootinfo);

    exception::init();
    memory::init(&bootinfo.memory);

    // TODO: reclaim boot memory
    
    log!("made it to the end!");
    aarch64::halt();
}

unsafe fn init_logging(uart_info: &info::Uart) {
    let uart = match uart_info {
        info::Uart::Pl011 { base } => {
            let base = pa_to_va(*base);
            Uart::pl011(base.as_mut_ptr())
        }
        info::Uart::Uart16550 { base } => {
            let base = pa_to_va(*base);
            Uart::uart16550(base.as_mut_ptr())
        }
    };

    log::init(uart);
}

fn print_bootinfo(bootinfo: &BootInfo) {
    let BootInfo {
        memory,
        uart,
        acpi_rsdp,
    } = bootinfo;

    log!("bootinfo.memory:");
    log!("     start        pages    type");
    log!("  ------------------------------");
    for block in &memory.blocks {
        log!("  {:#012}  {:8}  {}", block.start, block.pages, block.type_);
    }
    log!("bootinfo.uart: {uart:?}");
    log!("bootinfo.acpi_rsdp: {acpi_rsdp:#}");
}
