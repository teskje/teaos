use core::arch::asm;
use core::fmt;

pub(super) struct Esr(u64);

impl Esr {
    pub fn load() -> Self {
        let esr: u64;
        unsafe {
            asm!("MRS {x}, ESR_EL1", x = out(reg) esr);
        }

        Self(esr)
    }

    pub fn ec(&self) -> ExcClass {
        let ec = (self.0 >> 26) & 0x3f;
        ExcClass::from(ec as u8)
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
            .field("ec", &format_args!("{:?}", self.ec()))
            .field("iss", &format_args!("{:#x}", self.iss()))
            .finish()
    }
}

#[derive(Debug)]
pub(super) enum ExcClass {
    Brk,
    Other(u8),
}

impl From<u8> for ExcClass {
    fn from(ec: u8) -> Self {
        match ec {
            0b111100 => Self::Brk,
            other => Self::Other(other)
        }
    }
}
