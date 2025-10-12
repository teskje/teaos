//! Print logging support.

use core::fmt::{self, Write};

use crate::memory::mmio;
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

/// Initialize kernel logging.
///
/// # Safety
///
/// The given UART configuration must be correct.
pub unsafe fn init(uart_info: boot_info::Uart) {
    let mmio = unsafe { mmio::claim_page(uart_info.base()) };
    let uart = match uart_info {
        boot_info::Uart::Pl011 { .. } => unsafe { Uart::pl011(mmio) },
        boot_info::Uart::Uart16550 { .. } => unsafe { Uart::uart16550(mmio) },
    };

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
pub fn log_args(args: fmt::Arguments, module: &str) {
    let time = aarch64::uptime().as_millis();
    unsafe {
        let logger = &raw mut LOGGER;
        writeln!(&mut *logger, "{time} [{module}] {args}").unwrap();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let module = module_path!();
        $crate::log::log_args(format_args!($($arg)*), module);
    }};
}
