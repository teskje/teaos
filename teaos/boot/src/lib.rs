//! The TeaOS boot loader.
//!
//! The boot loader is really just a thin shim between UEFI and the TeaOS kernel. It presents as a
//! UEFI application that loads the kernel from the boot disk, collects information about the
//! system required for the kernel to boot, then exits boot services and jumps into the kernel.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod log;

mod acpi;
mod allocator;
mod paging;
mod uefi;

use aarch64::memory::paging::{MemoryClass, PAGE_SIZE, load_ttbr1};
use aarch64::memory::{PA, VA};
use alloc::vec;
use alloc::vec::Vec;
use boot_info::{BootInfo, MemoryType};
use core::ffi::c_void;
use core::mem;
use elf::ElfFile;

use crate::paging::PageMap;

/// Initialize the UEFI wrapper.
///
/// # Safety
///
/// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
pub unsafe fn init_uefi(image_handle: *mut c_void, system_table: *mut c_void) {
    unsafe { uefi::init(image_handle, system_table.cast()) }
}

/// Run the boot loader.
///
/// This loads the kernel binary, retrieves all required boot information, and finally passes
/// control to the kernel.
pub fn load() -> ! {
    log!("entered UEFI boot loader");

    log!("loading kernel binary");
    let mut kernel = load_kernel();
    log!("  kernel.entry={:#?}", kernel.entry);
    log!("  kernel.phys_start={:?}", kernel.phys_start);

    log!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp();
    log!("  rsdp_ptr={rsdp:#?}");

    log!("retrieving UART config");
    let uart_info = unsafe { find_uart(rsdp) };
    log!("  uart={uart_info:?}");

    log!("creating phys mapping");
    let uart_base = uart_info.base();
    create_phys_mapping(&mut kernel.kernel_map, kernel.phys_start, uart_base);

    log!("exiting boot services");
    let memory_info = exit_boot_services();

    // No (de)allocating or logging beyond this point!
    // We have lost access to the boot services and any attempt to invoke one will panic.

    // SAFETY: Only ttbr0 translation is used up to this point.
    unsafe { load_ttbr1(&kernel.kernel_map) };

    let bootinfo = BootInfo {
        memory: memory_info,
        uart: uart_info,
        acpi_rsdp: PA::new(rsdp as u64),
    }
    .into_ffi();

    (kernel.entry)(bootinfo);
}

struct Kernel {
    entry: fn(boot_info::ffi::BootInfo) -> !,
    kernel_map: PageMap,
    phys_start: VA,
}

/// Memory type used by the loader for pages containing kernel code or data.
const KERNEL_MEMORY: uefi::sys::MEMORY_TYPE = 0x80000000;

/// Load the kernel binary.
///
/// The kernel binary is expected to be located in the boot file system at `\kernel`, and is
/// expected to be an ELF file. Its loadable segments are read into memory and mapped into the
/// returned page table.
fn load_kernel() -> Kernel {
    let boot_fs = uefi::get_boot_fs();
    let root = boot_fs.open_volume();
    let kernel_file = root.open("\\kernel");

    let mut elf = ElfFile::open(kernel_file);

    let entry = elf.entry();
    let entry = unsafe { mem::transmute::<usize, fn(boot_info::ffi::BootInfo) -> !>(entry) };

    let mut kernel_map = PageMap::new();
    let phdrs: Vec<_> = elf.program_headers().collect();
    for phdr in phdrs {
        if !phdr.is_load() {
            continue;
        }

        let size = phdr.memory_size() as usize;
        let buffer = uefi::allocate_page_memory(size, KERNEL_MEMORY);
        elf.read_segment(&phdr, buffer);

        let pa = PA::new(buffer.as_ptr() as u64);
        let va = VA::new(phdr.virtual_address());
        let size = buffer.len();
        let class = MemoryClass::Normal;
        kernel_map.map_region(va, pa, size, class);
        log!("  mapped {va:#} -> {pa:#} ({size:#x} bytes, {class:?})");
    }

    let mut phys_start = None;
    if let Some(strtab) = elf.symbol_strtab() {
        for sym in elf.symbols().unwrap() {
            if sym.name(&strtab) == c"phys_start" {
                phys_start = Some(VA::new(sym.value()));
                break;
            }
        }
    }

    let phys_start = phys_start.expect("missing `phys_start` kernel symbol");

    Kernel {
        entry,
        kernel_map,
        phys_start,
    }
}

fn create_phys_mapping(kernel_map: &mut PageMap, phys_start: VA, uart_base: PA) {
    let (buffer_size, _) = uefi::get_memory_map_size();
    // Allocating this `Vec` may add an entry to the memory map, so we need to overprovision.
    let buffer = vec![0; buffer_size + 1024];
    let memory_map = uefi::get_memory_map(buffer);

    for desc in memory_map.iter() {
        let Some(block) = memory_bootinfo_from_uefi(desc) else {
            continue;
        };

        let pa = block.start;
        let va = phys_start + pa.into_u64();
        let pages = block.pages;
        let size = pages * PAGE_SIZE;
        let class = match block.type_ {
            MemoryType::Unused | MemoryType::Boot | MemoryType::Acpi | MemoryType::Kernel => {
                MemoryClass::Normal
            }
            MemoryType::Mmio => MemoryClass::Device,
        };
        kernel_map.map_region(va, pa, size, class);
        log!("  mapped {va:#} -> {pa:#} ({size:#x} bytes, {class:?})");
    }

    // The UEFI memory map doesn't include all device MMIO regions, so map the UART one explicitly.
    let va = phys_start + u64::from(uart_base);
    let class = MemoryClass::Device;
    kernel_map.map_page(va, uart_base, class);
    log!("  mapped {va:#} -> {uart_base:#} ({PAGE_SIZE:#x} bytes, {class:?})");
}

/// Find the ACPI RSDP in the UEFI config table.
///
/// # Panics
///
/// Panics if no RSDP entry is found.
fn find_acpi_rsdp() -> *mut acpi::RSDP {
    for (guid, ptr) in uefi::config_table().iter() {
        if guid == uefi::sys::ACPI_TABLE_GUID {
            return ptr.cast();
        }
    }

    panic!("ACPI config table not found");
}

/// Retrieve information about the serial port.
///
/// Finds the SPCR in the ACPI tables and extracts the UART type and base address.
///
/// # Safety
///
/// `rsdp` must be a valid pointer to an [`acpi::RSDP`].
unsafe fn find_uart(rsdp_ptr: *mut acpi::RSDP) -> boot_info::Uart {
    let rsdp = unsafe { &*rsdp_ptr };

    assert_eq!(rsdp.signature, *b"RSD PTR ");
    assert_eq!(rsdp.revision, 2);

    let xsdt_ptr = rsdp.xsdt_address;
    let xsdt = unsafe { &*xsdt_ptr };
    assert_eq!(xsdt.header.signature, *b"XSDT");
    assert_eq!(xsdt.header.revision, 1);

    let xsdt_size = xsdt.header.length as usize;
    let mut entry_size = xsdt_size - mem::size_of::<acpi::XSDT>();
    let mut entry_ptr = xsdt.entry.as_ptr();

    let mut spcr: Option<&acpi::SPCR> = None;
    const ADDR_SIZE: usize = mem::size_of::<usize>();
    while entry_size >= ADDR_SIZE {
        let addr_bytes_ptr = entry_ptr as *mut [u8; ADDR_SIZE];
        let addr_bytes = unsafe { *addr_bytes_ptr };
        let addr = usize::from_le_bytes(addr_bytes);
        let desc_ptr = addr as *mut acpi::DESCRIPTION_HEADER;
        let desc = unsafe { &*desc_ptr };
        if desc.signature == *b"SPCR" {
            spcr = Some(unsafe { &*desc_ptr.cast() });
            break;
        }

        entry_ptr = unsafe { entry_ptr.add(ADDR_SIZE) };
        entry_size -= ADDR_SIZE;
    }

    let spcr = spcr.expect("SPCR table present");
    assert_eq!(spcr.header.revision, 2);

    let base = spcr.base_address.address;

    match spcr.interface_type {
        acpi::UART_TYPE_16550 => boot_info::Uart::Uart16550 { base },
        acpi::UART_TYPE_PL011 => boot_info::Uart::Pl011 { base },
        value => unimplemented!("UART type: {value:#x}"),
    }
}

/// Exit the UEFI boot services.
///
/// Returns information about the physical memory in the system.
fn exit_boot_services() -> boot_info::Memory<'static> {
    let (buffer_size, desc_size) = uefi::get_memory_map_size();
    let len = buffer_size / desc_size;

    // Allocating these `Vec`s may add entries to the memory map, so we need to overprovision.
    let buffer = vec![0; buffer_size + 1024];
    let mut block_info = Vec::with_capacity(len + 5);

    let memory_map = uefi::get_memory_map(buffer);

    uefi::exit_boot_services(memory_map.map_key);

    for desc in memory_map.iter() {
        if let Some(block) = memory_bootinfo_from_uefi(desc) {
            block_info.push(block);
        }
    }

    // We can't deallocate anymore, so we must avoid dropping the `MemoryMap`.
    mem::forget(memory_map);

    boot_info::Memory::new(block_info)
}

fn memory_bootinfo_from_uefi(
    desc: &uefi::sys::MEMORY_DESCRIPTOR,
) -> Option<boot_info::MemoryBlock> {
    use uefi::sys::*;

    #[allow(non_upper_case_globals)]
    let type_ = match desc.type_ {
        ConventionalMemory | PersistentMemory => MemoryType::Unused,
        LoaderCode | LoaderData | BootServicesCode | BootServicesData | RuntimeServicesCode
        | RuntimeServicesData => MemoryType::Boot,
        ACPIReclaimMemory | ACPIMemoryNVS => MemoryType::Acpi,
        MemoryMappedIO | MemoryMappedIOPortSpace => MemoryType::Mmio,
        KERNEL_MEMORY => MemoryType::Kernel,
        ReservedMemoryType | UnusableMemory | PalCode | UnacceptedMemoryType => return None,
        _ => return None,
    };

    let block = boot_info::MemoryBlock {
        type_,
        start: desc.physical_start.into(),
        pages: desc.number_of_pages as usize,
    };
    Some(block)
}

/// Validate the given pointer.
///
/// # Panics
///
/// Panics if the given pointer is NULL.
/// Panics if the given pointer is not correctly aligned.
fn validate_ptr<T>(ptr: *const T) {
    assert!(!ptr.is_null());
    assert!(ptr.is_aligned());
}

/// Validate the given pointer.
///
/// # Panics
///
/// Panics if the given pointer is NULL.
/// Panics if the given pointer is not correctly aligned.
fn validate_mut_ptr<T>(ptr: *mut T) {
    assert!(!ptr.is_null());
    assert!(ptr.is_aligned());
}
