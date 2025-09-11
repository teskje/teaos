#![no_std]

pub mod instruction;
pub mod memory;
pub mod register;

use core::hint;
use core::time::Duration;

use instruction::wfe;
use register::{CNTFRQ_EL0, CNTVCT_EL0};

/// Halt the CPU indefinitely.
pub fn halt() -> ! {
    loop {
        wfe();
    }
}

/// Return the CPU uptime.
pub fn uptime() -> Duration {
    let count = CNTVCT_EL0::read().VirtualCount();
    let freq = CNTFRQ_EL0::read().ClockFreq();
    Duration::from_millis(count * 1_000 / freq)
}

pub fn delay(period: Duration) {
    let end = uptime() + period;
    while uptime() < end {
        hint::spin_loop();
    }
}
