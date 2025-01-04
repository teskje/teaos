#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

use tos_boot::info::BootInfo;
use tos_teaos::println;

#[no_mangle]
pub fn _start(boot_info: &BootInfo) -> ! {
    tos_teaos::kernel(boot_info);
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    println!("PANIC: {}", panic.message());
    if let Some(loc) = panic.location() {
        println!("  in file '{}' at line {}", loc.file(), loc.line());
    }

    loop {
        wfe();
    }
}

fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, preserves_flags, nostack));
    }
}
