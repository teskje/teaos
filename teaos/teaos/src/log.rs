//! Print logging support.

use core::fmt;

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

impl fmt::Write for Logger {
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

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let time = aarch64::uptime();
        let module = module_path!();
        $crate::log::write(format_args!("{time} [{module}] "));
        $crate::log::write(format_args!($($arg)*));
        $crate::log::write(format_args!("\n"));
    }};
}
