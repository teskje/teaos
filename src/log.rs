use core::{fmt, ptr};

use crate::uefi;

static mut LOGGER: Logger = Logger::None;

enum Logger {
    None,
    Uefi(uefi::ConOut),
}

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Uefi(out) => out.write_str(s),
        }
    }
}

pub fn set_uefi(out: uefi::ConOut) {
    // SAFETY: single-threaded access and only short-lived references
    unsafe {
        let logger = &mut *ptr::addr_of_mut!(LOGGER);
        *logger = Logger::Uefi(out);
    }
}

pub fn write(args: fmt::Arguments) {
    // SAFETY: single-threaded access and only short-lived references
    unsafe {
        let logger = &mut *ptr::addr_of_mut!(LOGGER);
        fmt::write(logger, args).unwrap();
    }
}

macro_rules! println {
    ($($arg:tt)*) => {{
        let time = $crate::cpu::virtual_time();
        let module = module_path!();
        $crate::log::write(format_args!("{time:.4} [{module}]   "));
        $crate::log::write(format_args!($($arg)*));
        $crate::log::write(format_args!("\n"));
    }};
}

pub(crate) use println;
