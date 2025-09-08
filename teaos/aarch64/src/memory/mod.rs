pub mod paging;

use core::arch::asm;
use core::fmt::{self, LowerHex};
use core::ops::{Add, AddAssign};

use crate::instruction::isb;
use crate::register::PAR_EL1;

/// Type for physical memory addresses.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PA(u64);

impl PA {
    pub const fn new(x: u64) -> Self {
        Self(x)
    }

    pub fn is_aligned_to(&self, x: usize) -> bool {
        usize::from(*self) % x == 0
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        usize::from(*self) as *mut _
    }
}

impl From<u64> for PA {
    fn from(x: u64) -> Self {
        Self(x)
    }
}

impl From<usize> for PA {
    fn from(x: usize) -> Self {
        Self(x.try_into().unwrap())
    }
}

impl From<PA> for u64 {
    fn from(pa: PA) -> Self {
        pa.0
    }
}

impl From<PA> for usize {
    fn from(pa: PA) -> Self {
        pa.0.try_into().unwrap()
    }
}

impl fmt::Debug for PA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PA({self:#})")
    }
}

impl fmt::Display for PA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Add<u64> for PA {
    type Output = Self;

    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl Add<usize> for PA {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + u64::try_from(rhs).unwrap())
    }
}

impl AddAssign<u64> for PA {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl AddAssign<usize> for PA {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

/// Type for virtual memory addresses.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VA(u64);

impl VA {
    pub const fn new(x: u64) -> Self {
        Self(x)
    }

    pub const fn into_u64(self) -> u64 {
        self.0
    }

    pub fn is_aligned_to(&self, x: usize) -> bool {
        usize::from(*self) % x == 0
    }

    pub fn as_ptr<T>(&self) -> *const T {
        usize::from(*self) as *const _
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        usize::from(*self) as *mut _
    }
}

impl From<u64> for VA {
    fn from(x: u64) -> Self {
        Self(x)
    }
}

impl From<usize> for VA {
    fn from(x: usize) -> Self {
        Self(x.try_into().unwrap())
    }
}

impl<T> From<&T> for VA {
    fn from(x: &T) -> Self {
        Self(x as *const _ as u64)
    }
}

impl From<VA> for u64 {
    fn from(va: VA) -> Self {
        va.0
    }
}

impl From<VA> for usize {
    fn from(va: VA) -> Self {
        va.0.try_into().unwrap()
    }
}

impl fmt::Debug for VA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VA({self:#})")
    }
}

impl fmt::Display for VA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Add<u64> for VA {
    type Output = Self;

    fn add(self, rhs: u64) -> Self {
        Self(self.0 + rhs)
    }
}

impl Add<usize> for VA {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + u64::try_from(rhs).unwrap())
    }
}

impl AddAssign<u64> for VA {
    fn add_assign(&mut self, rhs: u64) {
        *self = *self + rhs;
    }
}

impl AddAssign<usize> for VA {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

/// Translate the given virtual address to a physical address.
pub fn va_to_pa(va: VA) -> PA {
    let va = u64::from(va);
    unsafe {
        asm!("at s1e1r, {x}", x = in(reg) va);
    }
    isb();

    let par = PAR_EL1::read();
    if par.F() != 0 {
        panic!(
            "address translation failed\n\
             PAR = {par:#?}"
        );
    }

    let pa = (par.PA() << 12) | (va & 0xfff);
    PA::new(pa)
}
