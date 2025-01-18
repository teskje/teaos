#![no_std]

pub mod instruction;
pub mod memory;
pub mod register;

use instruction::wfe;
use register::{CNTFRQ_EL0, CNTVCT_EL0};

/// Halt the CPU indefinitely.
pub fn halt() -> ! {
    loop {
        wfe();
    }
}

/// Return the CPU uptime, in milliseconds.
pub fn uptime() -> u64 {
    let count = CNTVCT_EL0::read().VirtualCount();
    let freq = CNTFRQ_EL0::read().ClockFreq();
    count * 1_000 / freq
}
