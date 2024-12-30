use core::arch::asm;
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
    // SAFETY: no other references to `LOGGER` exist
    unsafe {
        let logger = &raw mut LOGGER;
        (*logger).uart = Some(uart);
    }
}

pub fn write(args: fmt::Arguments) {
    // SAFETY: no other references to `LOGGER` exist
    unsafe {
        let logger = &raw mut LOGGER;
        fmt::write(&mut *logger, args).unwrap();
    }
}

pub fn virtual_time() -> f64 {
    let count: u64;
    let freq: u64;
    unsafe {
        asm!(
            "mrs {cnt}, cntvct_el0",
            "mrs {frq}, cntfrq_el0",
            cnt = out(reg) count,
            frq = out(reg) freq,
            options(nomem, preserves_flags, nostack),
        );
    }

    count as f64 / freq as f64
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let time = $crate::log::virtual_time();
        let module = module_path!();
        $crate::log::write(format_args!("{time:.4} [{module}]   "));
        $crate::log::write(format_args!($($arg)*));
        $crate::log::write(format_args!("\n"));
    }};
}
