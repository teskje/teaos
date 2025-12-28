#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;

use sys::syscall;

#[unsafe(no_mangle)]
pub fn _start(heap_start: *mut u8, heap_size: usize) -> ! {
    unsafe { sys::heap::init(heap_start, heap_size) };

    let s = format!("heap_start={heap_start:?}, heap_size={heap_size:#x}");
    syscall::print(&s);
    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}
