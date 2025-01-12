mod esr;

use core::arch::{asm, global_asm};

use esr::Esr;

use crate::println;

extern "C" {
    #[link_name = "exception_vectors"]
    static EXCEPTION_VECTORS: u8;
}

#[no_mangle]
static mut OVERFLOW_STACK: [u8; 4096] = [0; 4096];

global_asm!(include_str!("vector.S"));

/// Initialize exception handling.
pub fn init() {
    let vector_base = unsafe { &EXCEPTION_VECTORS as *const _ as u64 };

    unsafe {
        asm!(
            "MSR VBAR_EL1, {addr}",
            "ISB",
            addr = in(reg) vector_base
        );
    }

    unsafe {
        asm!("brk 1");
    }
}

#[no_mangle]
pub extern "C" fn default_handler() {
    let esr = Esr::load();

    match esr.ec() {
        esr::ExcClass::Brk => handle_breakpoint(),
        esr::ExcClass::Other(_) => panic!("unhandled exception, ESR = {esr:?}"),
    }
}

fn handle_breakpoint() {
    println!("handling breakpoint exception");

    unsafe {
        asm!(
            "MRS {x}, ELR_EL1",
            "ADD {x}, {x}, #4",
            "MSR ELR_EL1, {x}",
            x = out(reg) _,
        );
    }
}
