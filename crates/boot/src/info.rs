//! The [`BootInfo`] passed to the kernel once loading is complete.

use alloc::vec::Vec;
use core::ffi::c_void;

use crate::uefi;

#[derive(Debug)]
pub struct BootInfo {
    pub memory: Memory,
    pub uart: Uart,
    pub rsdp: *mut c_void,
}

#[derive(Debug)]
pub struct Memory {
    pub blocks: Vec<MemoryBlock>,
}

#[derive(Debug)]
pub struct MemoryBlock {
    pub type_: MemoryType,
    pub start: u64,
    pub pages: u64,
}

#[derive(Debug)]
pub enum MemoryType {
    /// Unused memory: can be freely used.
    Unused,
    /// Memory used by the boot loader: can be reclaimed once the kernel has fully taken over.
    ///
    /// Usage of this memory type includes, but is not limited to:
    ///  * the `BootInfo`
    ///  * the initial stack
    ///  * the initial page tables
    Loader,
    /// Memory containing ACPI structures.
    Acpi,
    /// Memory containing memory-maped I/O registers.
    Mmio,
}

impl TryFrom<uefi::sys::MEMORY_TYPE> for MemoryType {
    type Error = ();

    fn try_from(type_: uefi::sys::MEMORY_TYPE) -> Result<Self, Self::Error> {
        use uefi::sys::*;

        #[allow(non_upper_case_globals)]
        match type_ {
            ConventionalMemory | PersistentMemory => Ok(MemoryType::Unused),
            LoaderCode | LoaderData | BootServicesCode | BootServicesData | RuntimeServicesCode
            | RuntimeServicesData => Ok(MemoryType::Loader),
            ACPIReclaimMemory | ACPIMemoryNVS => Ok(MemoryType::Acpi),
            MemoryMappedIO | MemoryMappedIOPortSpace => Ok(MemoryType::Mmio),
            ReservedMemoryType | UnusableMemory | PalCode | UnacceptedMemoryType | _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Uart {
    Pl011 { base: *mut c_void },
    Uart16550 { base: *mut c_void },
}
