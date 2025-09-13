mod id;

use aarch64::memory::paging::MemoryClass;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::mem;
use core::ops::RangeInclusive;

use aarch64::memory::PA;

use crate::log;
use crate::memory::{map_page, pa_to_va};

#[derive(Debug)]
pub struct Device {
    sbdf: Sbdf,
    config_space: *const [u32; 4096],
}

impl Device {
    /// # Safety
    ///
    /// `config_space` must be a valid pointer to a PCIe config space.
    unsafe fn new(sbdf: Sbdf, config_space: *const [u32; 4096]) -> Self {
        Self { sbdf, config_space }
    }

    fn vendor_id(&self) -> u16 {
        let word = unsafe { (*self.config_space)[0] };
        word as u16
    }

    fn device_id(&self) -> u16 {
        let word = unsafe { (*self.config_space)[0] };
        (word >> 16) as u16
    }

    fn multi_function(&self) -> bool {
        let word = unsafe { (*self.config_space)[3] };
        word & (1 << 23) != 0
    }
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.sbdf)?;

        let vendor_id = self.vendor_id();
        write!(f, " vendor_id={vendor_id:04x}")?;
        if let Some(vendor) = id::vendor(vendor_id) {
            write!(f, " ({vendor})")?;
        }

        let device_id = self.device_id();
        write!(f, ", device_id={device_id:04x}")?;
        if let Some(device) = id::device(vendor_id, device_id) {
            write!(f, " ({device})")?;
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
        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{:x}",
            self.segment, self.bus, self.device, self.function,
        )
    }
}

/// # Safety
///
/// RSDP pointer must be valid, as must be all the referenced ACPI structures.
pub unsafe fn discover(acpi_rsdp: *const acpi::RSDP) -> Vec<Device> {
    log!("discovering PCI devices");

    let allocations = unsafe { acpi_find_config_spaces(acpi_rsdp) };

    let mut devices = Vec::new();
    for alloc in allocations {
        let mut devs = alloc.enumerate_devices();
        devices.append(&mut devs);
    }

    log!("  devices:");
    for dev in &devices {
        log!("  - {dev}");
    }

    devices
}

/// # Safety
///
/// RSDP pointer must be valid, as must be all the referenced ACPI structures.
unsafe fn acpi_find_config_spaces(acpi_rsdp: *const acpi::RSDP) -> Vec<ConfigSpaceAllocation> {
    let rsdp = unsafe { &*acpi_rsdp };

    assert_eq!(rsdp.signature, *b"RSD PTR ");
    assert_eq!(rsdp.revision, 2);

    let xsdt_pa = PA::new(rsdp.xsdt_address);
    let xsdt_ptr: *const acpi::XSDT = pa_to_va(xsdt_pa).as_ptr();
    let xsdt = unsafe { &*xsdt_ptr };
    assert_eq!(xsdt.header.signature, *b"XSDT");
    assert_eq!(xsdt.header.revision, 1);

    let xsdt_size = xsdt.header.length as usize;
    let mut entry_size = xsdt_size - mem::offset_of!(acpi::XSDT, entry);
    let mut entry_ptr = xsdt.entry.as_ptr();

    let mut mcfg: Option<&acpi::MCFG> = None;
    const ADDR_SIZE: usize = mem::size_of::<usize>();
    while entry_size >= ADDR_SIZE {
        let addr_bytes_ptr = entry_ptr as *mut [u8; ADDR_SIZE];
        let addr_bytes = unsafe { *addr_bytes_ptr };
        let addr = u64::from_le_bytes(addr_bytes);

        let desc_pa = PA::new(addr);
        let desc_ptr: *const acpi::DESCRIPTION_HEADER = pa_to_va(desc_pa).as_ptr();
        let desc = unsafe { &*desc_ptr };

        if desc.signature == *b"MCFG" {
            mcfg = Some(unsafe { &*desc_ptr.cast() });
            break;
        }

        entry_ptr = unsafe { entry_ptr.add(ADDR_SIZE) };
        entry_size -= ADDR_SIZE;
    }

    let mcfg = mcfg.expect("MCFG table present");
    assert!(mcfg.header.revision == 1 || mcfg.header.revision == 2);

    let mcfg_size = mcfg.header.length as usize;
    let allocations_size = mcfg_size - mem::offset_of!(acpi::MCFG, allocations);
    let mut allocation_ptr: *const acpi::MCFG_Allocation = mcfg.allocations.as_ptr().cast();

    const ALLOCATION_SIZE: usize = mem::size_of::<acpi::MCFG_Allocation>();
    let num_allocations = allocations_size / ALLOCATION_SIZE;

    let mut allocations = Vec::with_capacity(num_allocations);
    for _ in 0..num_allocations {
        let allocation: &acpi::MCFG_Allocation = unsafe { &*allocation_ptr };

        allocations.push(ConfigSpaceAllocation {
            segment: allocation.segment,
            bus_range: allocation.start_bus_number..=allocation.end_bus_number,
            base: PA::new(allocation.base_address),
        });

        allocation_ptr = unsafe { allocation_ptr.add(1) };
    }

    allocations
}

#[derive(Debug)]
struct ConfigSpaceAllocation {
    segment: u16,
    bus_range: RangeInclusive<u8>,
    base: PA,
}

impl ConfigSpaceAllocation {
    fn enumerate_devices(self) -> Vec<Device> {
        let mut devices = Vec::new();
        for bus_nr in self.bus_range.clone() {
            for device_nr in 0..32 {
                let mut devs = self.probe_device(bus_nr, device_nr);
                devices.append(&mut devs);
            }
        }

        devices
    }

    fn probe_device(&self, bus_nr: u8, dev_nr: u8) -> Vec<Device> {
        let Some(dev) = self.probe_function(bus_nr, dev_nr, 0) else {
            return Vec::new();
        };

        if !dev.multi_function() {
            return vec![dev];
        }

        let mut devices = vec![dev];
        for fn_nr in 1..8 {
            if let Some(dev) = self.probe_function(bus_nr, dev_nr, fn_nr) {
                devices.push(dev);
            }
        }

        devices
    }

    fn probe_function(&self, bus_nr: u8, dev_nr: u8, fn_nr: u8) -> Option<Device> {
        let offset = u64::from(bus_nr) << 20 | u64::from(dev_nr) << 15 | u64::from(fn_nr) << 12;
        let pa = self.base + offset;
        let va = pa_to_va(pa);
        map_page(va, pa, MemoryClass::Device);

        let sbdf = Sbdf {
            segment: self.segment,
            bus: bus_nr,
            device: dev_nr,
            function: fn_nr,
        };
        let dev = unsafe { Device::new(sbdf, va.as_ptr()) };

        if dev.vendor_id() == 0xffff {
            None
        } else {
            Some(dev)
        }
    }
}
