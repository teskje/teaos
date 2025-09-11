#![no_std]
#![no_main]

use core::panic::PanicInfo;

use boot::info::BootInfo;
use teaos::log;

/// # Safety
///
/// The provided `bootinfo` must contain correct memory addresses.
#[unsafe(no_mangle)]
pub unsafe fn _start(bootinfo: &BootInfo) -> ! {
    unsafe { teaos::start(bootinfo) }
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    log!("PANIC: {}", panic.message());
    if let Some(loc) = panic.location() {
        log!("  in file '{}' at line {}", loc.file(), loc.line());
    }

    aarch64::halt();
}
