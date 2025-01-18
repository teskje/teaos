#![no_std]
#![no_main]

use core::panic::PanicInfo;

use boot::info::BootInfo;
use teaos::println;

/// # Safety
///
/// The provided `bootinfo` must contain correct memory addresses.
#[no_mangle]
pub unsafe fn _start(bootinfo: &BootInfo) -> ! {
    teaos::kernel(bootinfo);
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    println!("PANIC: {}", panic.message());
    if let Some(loc) = panic.location() {
        println!("  in file '{}' at line {}", loc.file(), loc.line());
    }

    aarch64::halt();
}
