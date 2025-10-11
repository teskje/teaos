//! Exception handling support.

mod syscall;

use core::arch::global_asm;

use aarch64::instruction::isb;
use aarch64::register::{ESR_EL1, FAR_EL1, VBAR_EL1};

use crate::log;

unsafe extern "C" {
    #[link_name = "exception_vectors"]
    static EXCEPTION_VECTORS: u8;
}

global_asm!(include_str!("vector.S"));

/// Initialize exception handling.
pub fn init() {
    log!("initializing exception handling");

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

#[unsafe(no_mangle)]
pub extern "C" fn handle_unhandled(stack: &mut ExceptionStack) {
    let esr = ESR_EL1::read();
    let far = FAR_EL1::read();

    panic!(
        "unhandled exception\n\
         ESR = {esr:#?}\n\
         FAR = {far:#?}\n\
         stack = {stack:#018x?}"
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_exception_el1(stack: &mut ExceptionStack) {
    let esr = ESR_EL1::read();

    match esr.EC() {
        0x3c => breakpoint(stack),
        ec => {
            log!("unhandled exception from EL1 (EC={ec})");
            handle_unhandled(stack);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_exception_el0(stack: &mut ExceptionStack) {
    let esr = ESR_EL1::read();

    match esr.EC() {
        0x15 => svc(stack),
        0x3c => breakpoint(stack),
        ec => {
            log!("unhandled exception from EL0 (EC={ec})");
            handle_unhandled(stack);
        }
    }
}

fn breakpoint(stack: &mut ExceptionStack) {
    log!("skipping breakpoint");
    stack.elr += 4;
}

fn svc(stack: &ExceptionStack) {
    let esr = ESR_EL1::read();
    let syscall_nr = esr.ISS() & 0xffff;

    match syscall_nr {
        0 => syscall::print(stack),
        _ => panic!("invalid syscall nr: {syscall_nr}"),
    }
}
