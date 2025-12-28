//! UEFI entry point for the TeaOS boot loader.

#![no_std]
#![no_main]

use core::ffi::c_void;
use core::panic::PanicInfo;

use boot::log;

#[unsafe(no_mangle)]
unsafe extern "efiapi" fn efi_main(image_handle: *mut c_void, system_table: *mut c_void) -> ! {
    unsafe { boot::init_uefi(image_handle, system_table) };
    boot::load();
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    log!("PANIC: {}", panic.message());
    if let Some(loc) = panic.location() {
        log!("  in file '{}' at line {}", loc.file(), loc.line());
    }

    aarch64::halt();
}
