//! The [`BootInfo`] passed to the kernel once loading is complete.

use core::ffi::c_void;

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub rsdp: *mut c_void,
    pub uart: Uart,
}

#[derive(Debug)]
#[repr(C)]
pub enum Uart {
    Pl011 { base: *mut c_void },
    Uart16550 { base: *mut c_void },
}
