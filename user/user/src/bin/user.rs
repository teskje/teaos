#![no_std]
#![no_main]

use core::panic::PanicInfo;

use user::sys_print;

#[unsafe(no_mangle)]
pub fn _start() -> ! {
    sys_print("hello from user mode");
    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
