//! Simple drivers for supported UART devices.

use core::{fmt, hint};

use crate::memory::mmio::MmioPage;

#[derive(Debug)]
pub enum Uart {
    Pl011(Pl011),
    Uart16550(Uart16550),
}

impl Uart {
    pub unsafe fn pl011(mmio: MmioPage) -> Self {
        Self::Pl011(Pl011 { mmio })
    }

    pub unsafe fn uart16550(mmio: MmioPage) -> Self {
        Self::Uart16550(Uart16550 { mmio })
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
    mmio: MmioPage,
}

impl Pl011 {
    fn write_dr(&mut self, val: u8) {
        unsafe { self.mmio.write(0x000, val) }
    }

    fn read_fr(&self) -> u16 {
        unsafe { self.mmio.read(0x018) }
    }

    fn busy(&self) -> bool {
        let flags = self.read_fr();
        flags & (1 << 3) != 0
    }
}

impl fmt::Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_dr(b);
            while self.busy() {
                hint::spin_loop();
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Uart16550 {
    mmio: MmioPage,
}

impl Uart16550 {
    fn write_thr(&mut self, val: u8) {
        unsafe { self.mmio.write(0b000, val) }
    }

    fn read_lsr(&self) -> u8 {
        unsafe { self.mmio.read(0b101) }
    }

    fn thr_empty(&self) -> bool {
        let flags = self.read_lsr();
        flags & (1 << 5) != 0
    }
}

impl fmt::Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            self.write_thr(b);
            while !self.thr_empty() {
                hint::spin_loop();
            }
        }
        Ok(())
    }
}
