//! The [`BootInfo`] passed to the kernel once loading is complete.

use core::fmt;

use alloc::vec::Vec;
use cpu::vmem::PAGE_SIZE;
use kstd::memory::PA;

use crate::uefi;

#[derive(Debug)]
pub struct BootInfo {
    pub memory: Memory,
    pub uart: Uart,
    pub acpi_rsdp: PA,
}

#[derive(Debug)]
pub struct Memory {
    pub blocks: Vec<MemoryBlock>,
}

impl Memory {
    pub(crate) fn new(mut blocks: Vec<MemoryBlock>) -> Self {
        // Cleanup: Merge consecutive blocks of the same type.
        blocks.sort_unstable_by_key(|b| b.start);

        fn can_merge(a: &MemoryBlock, b: &MemoryBlock) -> bool {
            let consequtive = a.start + a.pages * PAGE_SIZE == b.start;
            let same_type = a.type_ == b.type_;
            consequtive && same_type
        }

        let mut i = 0;
        while let (Some(cur), Some(next)) = (blocks.get(i), blocks.get(i + 1)) {
            if can_merge(cur, next) {
                blocks[i].pages += next.pages;
                blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }

        Self { blocks }
    }
}

#[derive(Debug)]
pub struct MemoryBlock {
    pub type_: MemoryType,
    pub start: PA,
    pub pages: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Unused => "unused",
            Self::Loader => "loader",
            Self::Acpi => "acpi",
            Self::Mmio => "mmio",
        };
        f.write_str(s)
    }
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
    Pl011 { base: PA },
    Uart16550 { base: PA },
}
