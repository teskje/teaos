//! Exception handling support.

use core::arch::global_asm;

use aarch64::instruction::isb;
use aarch64::register::{ESR_EL1, VBAR_EL1};

use crate::println;

extern "C" {
    #[link_name = "exception_vectors"]
    static EXCEPTION_VECTORS: u8;
}

global_asm!(include_str!("vector.S"));

/// Initialize exception handling.
pub fn init() {
    println!("initializing exception handling");

    unsafe {
        let vector_base = &EXCEPTION_VECTORS as *const _ as u64;
        VBAR_EL1::write(vector_base);
    }
    isb();
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
    let esr = ESR_EL1::read();

    panic!(
        "unhandled exception from EL1\n\
         ESR = {esr:#?}\n\
         stack = {stack:#018x?}"
    );
}

#[no_mangle]
pub extern "C" fn handle_exception_el1(stack: &mut ExceptionStack) {
    let esr = ESR_EL1::read();

    match esr.EC() {
        0x3c => breakpoint(stack),
        _ => handle_unhandled(stack),
    }
}

fn breakpoint(stack: &mut ExceptionStack) {
    println!("skipping breakpoint");
    stack.elr += 4;
}
