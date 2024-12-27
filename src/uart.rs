use core::ffi::c_void;
use core::fmt;

#[derive(Debug)]
pub enum Uart {
    Pl011(Pl011),
    Uart16550(Uart16550),
}

impl Uart {
    pub unsafe fn pl011(base: *mut c_void) -> Self {
        Self::Pl011(Pl011::new(base))
    }

    pub unsafe fn uart_16550(base: *mut c_void) -> Self {
        Self::Uart16550(Uart16550::new(base))
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Uart::Pl011(inner) => inner.write_str(s),
            Uart::Uart16550(inner) => inner.write_str(s),
        }
    }
}

#[derive(Debug)]
pub struct Pl011 {
    base: *mut c_void,
}

impl Pl011 {
    unsafe fn new(base: *mut c_void) -> Self {
        Self { base }
    }

    fn write_dr(&mut self, val: u8) {
        unsafe {
            let dr: *mut u8 = self.base.add(0x000).cast();
            dr.write_volatile(val);
        }
    }

    fn busy(&self) -> bool {
        unsafe {
            let fr: *mut u16 = self.base.add(0x018).cast();
            let flags = fr.read_volatile();
            flags & (1 << 3) != 0
        }
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_dr(b);
            while self.busy() {}
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Uart16550 {
    base: *mut c_void,
}

impl Uart16550 {
    unsafe fn new(base: *mut c_void) -> Self {
        Self { base }
    }

    fn write_thr(&mut self, val: u8) {
        unsafe {
            let thr: *mut u8 = self.base.add(0b000).cast();
            thr.write_volatile(val);
        }
    }

    fn thr_empty(&self) -> bool {
        unsafe {
            let lsr: *mut u8 = self.base.add(0b101).cast();
            let flags = lsr.read_volatile();
            flags & (1 << 5) != 0
        }
    }
}

impl fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_thr(b);
            while !self.thr_empty() {}
        }
        Ok(())
    }
}
