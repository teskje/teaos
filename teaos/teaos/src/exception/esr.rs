use core::arch::asm;
use core::fmt;

pub(super) struct Esr(u64);

impl Esr {
    pub fn read() -> Self {
        let esr: u64;
        unsafe {
            asm!("mrs {x}, esr_el1", x = out(reg) esr);
        }

        Self(esr)
    }

    pub fn ec(&self) -> u8 {
        let ec = (self.0 >> 26) & 0x3f;
        ec as u8
    }

    fn iss(&self) -> u32 {
        let iss = self.0 & 0x1ffffff;
        iss as u32
    }
}

impl fmt::Debug for Esr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ESR")
            .field("raw", &format_args!("{:#x}", self.0))
            .field("ec", &format_args!("{:#x}", self.ec()))
            .field("iss", &format_args!("{:#x}", self.iss()))
            .finish()
    }
}
