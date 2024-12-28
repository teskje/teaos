#![no_std]
#![no_main]

mod crc32;
mod cpu;
mod log;
mod uart;
mod uefi;

use core::ffi::c_void;
use core::panic::PanicInfo;

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

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    println!("PANIC: {panic:?}");
    loop {}
}
