use core::fmt;

use crate::uefi;

static mut LOGGER: Logger = Logger::new();

struct Logger {
    out: Option<uefi::ConsoleOut>,
}

impl Logger {
    const fn new() -> Self {
        Self { out: None }
    }
}

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(out) = &mut self.out {
            out.write_str(s)
        } else {
            Ok(())
        }
    }
}

pub fn init(out: uefi::ConsoleOut) {
    // SAFETY: no other references to `LOGGER` exist
    unsafe {
        let logger = &raw mut LOGGER;
        (*logger).out = Some(out);
    }
}

pub fn uninit() {
    // SAFETY: no other references to `LOGGER` exist
    unsafe {
        let logger = &raw mut LOGGER;
        (*logger).out = None;
    }
}

pub fn write(args: fmt::Arguments) {
    // SAFETY: no other references to `LOGGER` exist
    unsafe {
        let logger = &raw mut LOGGER;
        fmt::write(&mut *logger, args).unwrap();
    }
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::log::write(format_args!("[boot]   "));
        $crate::log::write(format_args!($($arg)*));
        $crate::log::write(format_args!("\n"));
    }};
}
