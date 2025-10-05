pub mod paging;

mod address;

use crate::instruction::{at_s1e1r, isb};
use crate::register::PAR_EL1;

pub use self::address::{PA, VA};

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MAP_LEVELS: u64 = 3;

pub fn va_to_pa(va: VA) -> Option<PA> {
    at_s1e1r(va);
    isb();

    let par = PAR_EL1::read();
    if par.F() != 0 {
        return None;
    }

    let offset = va.into_u64() & 0xfff;
    let pa = (par.PA() << 12) | offset;
    Some(PA::new(pa))
}
