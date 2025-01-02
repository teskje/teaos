#![cfg_attr(not(test), no_std)]

pub mod log;

mod uart;

use core::arch::asm;

use boot::info::{self, BootInfo};

use crate::uart::Uart;

pub fn kernel(boot_info: &BootInfo) -> ! {
    unsafe { init_logging(&boot_info.uart) };
    println!("kernel logging initialized");

    loop {
        wfe();
    }
}

unsafe fn init_logging(uart_info: &info::Uart) {
    let uart = match uart_info {
        info::Uart::Pl011 { base } => Uart::pl011(*base),
        info::Uart::Uart16550 { base } => Uart::uart16550(*base),
    };

    log::init(uart);
}

fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}
