//! Print logging support.

use core::fmt::{self, Write};

use crate::uefi;

#[inline(never)]
pub fn log_args(args: fmt::Arguments) {
    let time = aarch64::uptime().as_millis();
    let mut out = uefi::console_out();
    writeln!(&mut out, "{time} [boot] {args}").unwrap();
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::log::log_args(format_args!($($arg)*));
    }};
}
