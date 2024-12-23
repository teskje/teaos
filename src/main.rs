#![no_std]
#![no_main]

mod cpu;
mod log;
mod uefi;

use core::ffi::c_void;
use core::panic::PanicInfo;

use crate::log::println;

fn kernel_main(rsdp_ptr: *mut c_void) -> ! {
    loop {}
}

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
    println!("PANIC: {panic:?}");
    loop {}
}
