#![no_std]
#![no_main]

mod device_tree;
mod print;
mod serial;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::slice;

use crate::device_tree::DeviceTree;

extern "C" {
    static _dtb_start: u8;
    static _dtb_end: u8;

    pub fn start() -> !;
}

global_asm!(include_str!("start.S"));

#[no_mangle]
extern "C" fn main() -> ! {
    serial::init();

    let dtb_data = unsafe {
        let start = &_dtb_start as *const u8;
        let end = &_dtb_end as *const u8;
        let len = end.offset_from(start).try_into().unwrap();
        slice::from_raw_parts(start, len)
    };
    let dt = DeviceTree::new(dtb_data);

    for token in dt.tokens() {
        println!("token: {token:?}");
    }

    let memory_range = dt.find_memory();
    println!("memory_range: {memory_range:?}");

    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    println!("!!! KERNEL PANIC !!!");
    println!("{}", _panic);

    loop {}
}
