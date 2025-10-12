#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
pub fn _start() -> ! {
    loop {
        foo(5);
    }
}

fn foo(count: u32) {
    for _ in 0..count {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
