//! Print logging support.

use core::fmt::{self, Write};

use crate::uart::Uart;

static mut LOGGER: Logger = Logger::new();

struct Logger {
    uart: Option<Uart>,
}

impl Logger {
    const fn new() -> Self {
        Self { uart: None }
    }
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(uart) = &mut self.uart {
            uart.write_str(s)
        } else {
            Ok(())
        }
    }
}

pub fn init(uart: Uart) {
    unsafe {
        let logger = &raw mut LOGGER;
        (*logger).uart = Some(uart);
    }
}

pub fn write(args: fmt::Arguments) {
    unsafe {
        let logger = &raw mut LOGGER;
        fmt::write(&mut *logger, args).unwrap();
    }
}

#[inline(never)]
pub fn log_args(args: fmt::Arguments) {
    let time = aarch64::uptime();
    unsafe {
        let logger = &raw mut LOGGER;
        writeln!(&mut *logger, "{time} [boot] {args}").unwrap();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::log::log_args(format_args!($($arg)*));
    }};
}
