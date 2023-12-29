use core::ptr::{read_volatile, write_volatile};

// TODO: parse from the dtb
const BASE: usize = 0x9000000;
const UARTDR: *mut u8 = (BASE + 0x000) as _;
const UARTFR: *const u16 = (BASE + 0x018) as _;
const UARTCR: *mut u16 = (BASE + 0x030) as _;

pub fn init() {
    unsafe { write_volatile(UARTCR, 0x0101) };
}

pub fn write(s: &str) {
    for b in s.bytes() {
        write_byte(b);
    }
}

fn write_byte(b: u8) {
    while transmit_fifo_full() {}
    unsafe { write_volatile(UARTDR, b) };
}

fn transmit_fifo_full() -> bool {
    let flags = unsafe { read_volatile(UARTFR) };
    (flags & (1 << 5)) != 0
}
