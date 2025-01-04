#![no_std]
#![no_main]

use core::panic::PanicInfo;

use boot::info::BootInfo;
use teaos::println;

#[no_mangle]
pub fn _start(boot_info: &BootInfo) -> ! {
    teaos::kernel(boot_info);
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    println!("PANIC: {}", panic.message());
    if let Some(loc) = panic.location() {
        println!("  in file '{}' at line {}", loc.file(), loc.line());
    }

    cpu::halt();
}
