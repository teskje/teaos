use core::fmt::{self, Write};

use crate::serial;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::print::print_args(::core::format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        $crate::print!("\n")
    }};
    ($fmt:expr) => {{
        $crate::print!($fmt);
        $crate::print!("\n");
    }};
    ($fmt:expr, $($arg:tt)*) => {{
        $crate::print!($fmt, $($arg)*);
        $crate::print!("\n");
    }};
}

pub fn print_args(args: fmt::Arguments) {
    Writer.write_fmt(args).ok();
}

struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        serial::write(s);
        Ok(())
    }
}
