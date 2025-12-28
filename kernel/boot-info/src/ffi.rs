//! FFI-compatible versions of `BootInfo` types.

use core::slice;

use aarch64::memory::PA;

use crate::{MemoryBlock, Uart};

#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    memory: Memory,
    uart: Uart,
    acpi_rsdp: PA,
}

#[repr(C)]
#[derive(Debug)]
pub struct Memory {
    blocks_ptr: *const MemoryBlock,
    blocks_len: usize,
}

impl super::BootInfo<'_> {
    pub fn into_ffi(self) -> BootInfo {
        BootInfo {
            memory: self.memory.into_ffi(),
            uart: self.uart,
            acpi_rsdp: self.acpi_rsdp,
        }
    }

    /// # Safety
    ///
    /// All pointers in `ffi` must be valid.
    pub unsafe fn from_ffi(ffi: BootInfo) -> Self {
        let memory = unsafe { super::Memory::from_ffi(ffi.memory) };

        Self {
            memory,
            uart: ffi.uart,
            acpi_rsdp: ffi.acpi_rsdp,
        }
    }
}

impl super::Memory<'_> {
    pub fn into_ffi(self) -> Memory {
        Memory {
            blocks_ptr: self.blocks.as_ptr(),
            blocks_len: self.blocks.len(),
        }
    }

    /// # Safety
    ///
    /// All pointers in `ffi` must be valid.
    pub unsafe fn from_ffi(ffi: Memory) -> Self {
        let blocks = unsafe { slice::from_raw_parts(ffi.blocks_ptr, ffi.blocks_len) };

        Self { blocks }
    }
}
