mod esr;

use core::arch::{asm, global_asm};

use esr::Esr;

use crate::println;

extern "C" {
    #[link_name = "exception_vectors"]
    static EXCEPTION_VECTORS: u8;
}

global_asm!(include_str!("vector.S"));

/// Initialize exception handling.
pub fn init() {
    let vector_base = unsafe { &EXCEPTION_VECTORS as *const _ as u64 };

    unsafe {
        asm!(
            "msr vbar_el1, {addr}",
            "isb",
            addr = in(reg) vector_base
        );
    }
}

#[derive(Debug)]
#[repr(C, packed)]
pub(super) struct ExceptionStack {
    spsr: u64,
    elr: u64,

    x0: u64,
    x1: u64,
    x2: u64,
    x3: u64,
    x4: u64,
    x5: u64,
    x6: u64,
    x7: u64,
    x8: u64,
    x9: u64,
    x10: u64,
    x11: u64,
    x12: u64,
    x13: u64,
    x14: u64,
    x15: u64,
    x16: u64,
    x17: u64,
    x18: u64,
    x30: u64,
}

#[no_mangle]
pub extern "C" fn handle_unhandled(stack: &mut ExceptionStack) {
    let esr = Esr::read();

    panic!(
        "unhandled exception from EL1\n\
         ESR = {esr:#x?}\n\
         stack = {stack:#018x?}"
    );
}

#[no_mangle]
pub extern "C" fn handle_exception_el1(stack: &mut ExceptionStack) {
    let esr = Esr::read();

    match esr.ec() {
        0x3c => handle_breakpoint(stack),
        _ => handle_unhandled(stack),
    }
}

fn handle_breakpoint(stack: &mut ExceptionStack) {
    println!("skipping breakpoint");
    stack.elr += 4;
}
