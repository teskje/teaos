use core::fmt;
use core::ops::AddAssign;

/// Type for physical memory addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

impl fmt::Display for PA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl AddAssign<u64> for PA {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl AddAssign<usize> for PA {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += u64::try_from(rhs).unwrap();
    }
}

/// Type for virtual memory addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VA(u64);

impl VA {
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

impl fmt::Display for VA {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl AddAssign<u64> for VA {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl AddAssign<usize> for VA {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += u64::try_from(rhs).unwrap();
    }
}
