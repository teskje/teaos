mod discover;
mod id;

use alloc::vec::Vec;
use core::{fmt, mem};

use crate::log;
use crate::memory::mmio::MmioPage;
use crate::pci::discover::Discovery;

#[derive(Debug)]
pub struct Function {
    sbdf: Sbdf,
    config_space: MmioPage,
}

impl Function {
    /// # Safety
    ///
    /// `config_space` must point to a valid PCIe config space.
    unsafe fn new(sbdf: Sbdf, config_space: MmioPage) -> Self {
        Self { sbdf, config_space }
    }

    fn read_config_word(&self, idx: usize) -> u32 {
        assert!(idx < 1024);

        let offset = idx * mem::size_of::<u32>();
        unsafe { self.config_space.read(offset) }
    }

    fn vendor_id(&self) -> u16 {
        self.read_config_word(0) as u16
    }

    fn device_id(&self) -> u16 {
        let w = self.read_config_word(0);
        (w >> 16) as u16
    }

    fn programming_interface(&self) -> u8 {
        let w = self.read_config_word(2);
        (w >> 8) as u8
    }

    fn subclass(&self) -> u8 {
        let w = self.read_config_word(2);
        (w >> 16) as u8
    }

    fn class(&self) -> u8 {
        let w = self.read_config_word(2);
        (w >> 24) as u8
    }

    fn multi_function(&self) -> bool {
        let w = self.read_config_word(3);
        w & (1 << 23) != 0
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.sbdf)?;

        let vendor_id = self.vendor_id();
        if let Some(vendor) = id::vendor(vendor_id) {
            write!(f, " vendor='{vendor}'")?;
        } else {
            write!(f, " vendor={vendor_id:04x}")?;
        }

        let device_id = self.device_id();
        if let Some(device) = id::device(vendor_id, device_id) {
            write!(f, ", device='{device}'")?;
        } else {
            write!(f, ", device={device_id:04x}")?;
        }

        let class = self.class();
        let subclass = self.subclass();
        let prog_if = self.programming_interface();
        if let Some(class_name) = id::class(class, subclass, prog_if) {
            write!(f, ", class='{class_name}'")?;
        } else {
            write!(f, ", class={class:02x}{subclass:02x}{prog_if:02x}")?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Sbdf {
    segment: u16,
    bus: u8,
    device: u8,
    function: u8,
}

impl fmt::Display for Sbdf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segment == 0 {
            write!(
                f,
                "{:02x}:{:02x}.{:x}",
                self.bus, self.device, self.function,
            )
        } else {
            write!(
                f,
                "{:04x}:{:02x}:{:02x}.{:x}",
                self.segment, self.bus, self.device, self.function,
            )
        }
    }
}

/// # Safety
///
/// RSDP pointer must be valid, as must be all the referenced ACPI structures.
pub unsafe fn discover(acpi_rsdp: *const acpi::RSDP) -> Vec<Function> {
    log!("discovering PCI devices");

    let functions = unsafe { Discovery::new(acpi_rsdp).run() };
    for func in &functions {
        log!("  {func}");
    }

    functions
}
