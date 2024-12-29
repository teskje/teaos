#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod cpu;
mod crc32;
mod log;
#[cfg(not(test))]
mod panic;
mod uart;
mod uefi;

use core::ffi::c_void;

use crate::log::println;
use crate::uart::Uart;

struct BootConfig {
    rsdp: *mut c_void,
    uart: Uart,
}

fn kernel_main(boot_config: BootConfig) -> ! {
    log::set_uart(boot_config.uart);
    println!("entered kernel");

    loop {
        cpu::wfe();
    }
}

#[cfg(test)]
fn main() {}
