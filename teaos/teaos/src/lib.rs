#![cfg_attr(not(test), no_std)]

pub mod log;

mod uart;
mod memory;

use boot::info::{self, BootInfo};

use crate::uart::Uart;

pub fn kernel(boot_info: &BootInfo) -> ! {
    unsafe { init_logging(&boot_info.uart) };
    println!("kernel logging initialized");

    cpu::halt();
}

unsafe fn init_logging(uart_info: &info::Uart) {
    let uart = match uart_info {
        info::Uart::Pl011 { base } => Uart::pl011(*base),
        info::Uart::Uart16550 { base } => Uart::uart16550(*base),
    };

    log::init(uart);
}
