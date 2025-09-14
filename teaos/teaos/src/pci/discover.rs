//! PCI function discovery.

use core::mem;

use aarch64::memory::PA;
use aarch64::memory::paging::MemoryClass;
use alloc::vec::Vec;

use crate::memory::{map_page, pa_to_va};
use crate::pci::{Function, Sbdf};

pub(super) struct Discovery {
    acpi_rsdp: *const acpi::RSDP,
    functions: Vec<Function>,
}

impl Discovery {
    /// # Safety
    ///
    /// RSDP pointer must be valid, as must be all the referenced ACPI structures.
    pub unsafe fn new(acpi_rsdp: *const acpi::RSDP) -> Self {
        Self {
            acpi_rsdp,
            functions: Vec::new(),
        }
    }

    pub fn run(mut self) -> Vec<Function> {
        let allocations = self.find_config_allocations();
        for alloc in &allocations {
            self.enumerate_functions(alloc);
        }

        self.functions
    }

    fn find_config_allocations(&self) -> Vec<ConfigAllocation> {
        let rsdp = unsafe { &*self.acpi_rsdp };

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

            allocations.push(ConfigAllocation {
                segment: allocation.segment,
                start_bus: allocation.start_bus_number,
                end_bus: allocation.end_bus_number,
                base_address: PA::new(allocation.base_address),
            });

            allocation_ptr = unsafe { allocation_ptr.add(1) };
        }

        allocations
    }

    fn enumerate_functions(&mut self, alloc: &ConfigAllocation) {
        let mut cursor = alloc.cursor();
        while cursor.valid() {
            self.probe_device(&mut cursor);
            cursor.step_device();
        }
    }

    fn probe_device(&mut self, cursor: &mut Cursor<'_>) {
        let multi_fn = self.probe_function(cursor);
        if !multi_fn {
            return;
        }

        cursor.step_function();
        while cursor.valid() {
            self.probe_function(cursor);
            cursor.step_function();
        }
    }

    fn probe_function(&mut self, cursor: &mut Cursor<'_>) -> bool {
        let pa = cursor.config_address();
        let va = pa_to_va(pa);
        map_page(va, pa, MemoryClass::Device);

        let fun = unsafe { Function::new(cursor.sbdf(), va.as_ptr()) };

        if fun.vendor_id() == 0xffff {
            return false;
        }

        let multi_fn = fun.multi_function();
        self.functions.push(fun);

        multi_fn
    }
}

#[derive(Debug)]
struct ConfigAllocation {
    segment: u16,
    start_bus: u8,
    end_bus: u8,
    base_address: PA,
}

impl ConfigAllocation {
    fn cursor(&self) -> Cursor<'_> {
        Cursor {
            alloc: self,
            bus_nr: self.start_bus,
            dev_nr: 0,
            fun_nr: 0,
        }
    }
}

struct Cursor<'a> {
    alloc: &'a ConfigAllocation,
    bus_nr: u8,
    dev_nr: u8,
    fun_nr: u8,
}

impl Cursor<'_> {
    fn valid(&self) -> bool {
        self.bus_nr <= self.alloc.end_bus && self.dev_nr < 32 && self.fun_nr < 8
    }

    fn step_device(&mut self) {
        if self.dev_nr < 32 {
            self.dev_nr += 1;
            self.fun_nr = 0
        } else {
            self.bus_nr += 1;
            self.dev_nr = 0;
            self.fun_nr = 0;
        }
    }

    fn step_function(&mut self) {
        self.fun_nr += 1;
    }

    fn sbdf(&self) -> Sbdf {
        Sbdf {
            segment: self.alloc.segment,
            bus: self.bus_nr,
            device: self.dev_nr,
            function: self.fun_nr,
        }
    }

    fn config_address(&self) -> PA {
        let offset = u64::from(self.bus_nr) << 20
            | u64::from(self.dev_nr) << 15
            | u64::from(self.fun_nr) << 12;
        self.alloc.base_address + offset
    }
}
