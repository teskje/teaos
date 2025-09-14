use core::fmt::{self, LowerHex};
use core::ops::{Add, AddAssign};

use super::PAGE_SIZE;

/// Type for physical memory addresses.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PA(u64);

impl PA {
    pub const fn new(x: u64) -> Self {
        assert!(x < (1 << 48), "PA size greater than 48 bits");

        Self(x)
    }

    pub const fn into_u64(self) -> u64 {
        self.0
    }

    pub const fn is_aligned_to(&self, x: usize) -> bool {
        self.0 % x as u64 == 0
    }

    pub const fn is_page_aligned(&self) -> bool {
        self.is_aligned_to(PAGE_SIZE)
    }
}

impl From<u64> for PA {
    fn from(x: u64) -> Self {
        Self::new(x)
    }
}

impl From<PA> for u64 {
    fn from(pa: PA) -> Self {
        pa.into_u64()
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
        self + rhs as u64
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

    pub const fn is_aligned_to(&self, x: usize) -> bool {
        self.0 % x as u64 == 0
    }

    pub const fn is_page_aligned(&self) -> bool {
        self.is_aligned_to(PAGE_SIZE)
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const _
    }

    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut _
    }

    pub const fn page_table_idx(&self, level: u64) -> usize {
        let shift = 39 - 9 * level;
        let idx = (self.0 >> shift) & 0x1ff;
        idx as usize
    }
}

impl From<u64> for VA {
    fn from(x: u64) -> Self {
        Self(x)
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
        self + rhs as u64
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
