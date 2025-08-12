//! Print logging support.

use core::fmt;

use crate::uefi;

pub fn write(args: fmt::Arguments) {
    let mut out = uefi::console_out();
    fmt::write(&mut out, args).unwrap();
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let time = aarch64::uptime();
        $crate::log::write(format_args!("{time} [boot]  "));
        $crate::log::write(format_args!($($arg)*));
        $crate::log::write(format_args!("\n"));
    }};
}
