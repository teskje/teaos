#![cfg_attr(not(test), no_std)]

pub mod log;

use core::arch::asm;

use boot::info::BootInfo;

use crate::uart::Uart;

mod uart;

pub fn kernel(boot_info: BootInfo) -> ! {
    init_logging(&boot_info);
    println!("entered kernel");

    loop {
        wfe();
    }
}

fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}

fn init_logging(boot_info: &BootInfo) {
    use boot::info::Uart::*;

    let uart = match boot_info.uart {
        Pl011 { base } => unsafe { Uart::pl011(base) },
        Uart16550 { base } => unsafe { Uart::uart_16550(base) },
    };

    log::init(uart);
}
