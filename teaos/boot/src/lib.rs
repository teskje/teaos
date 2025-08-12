//! The TeaOS boot loader.
//!
//! The boot loader is really just a thin shim between UEFI and the TeaOS kernel. It presents as a
//! UEFI application that loads the kernel from the boot disk, collects information about the
//! system required for the kernel to boot, then exits boot services and jumps into the kernel.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod info;
pub mod log;

mod acpi;
mod allocator;
mod uefi;

use aarch64::memory::paging::TranslationTable;
use aarch64::memory::{PA, VA};
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_void;
use core::mem;

use elf::ElfFile;

use crate::info::BootInfo;

/// Initialize the UEFI wrapper.
///
/// # Safety
///
/// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
pub unsafe fn init_uefi(image_handle: *mut c_void, system_table: *mut c_void) {
    uefi::init(image_handle, system_table.cast());
}

/// Run the boot loader.
///
/// This loads the kernel binary, retrieves all required boot information, and finally passes
/// control to the kernel.
pub fn load() -> ! {
    println!("entered UEFI boot loader");

    println!("loading kernel binary");
    let (kernel_start, kernel_tt) = load_kernel();
    println!("  kernel_start={kernel_start:#?}");

    println!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp();
    println!("  rsdp_ptr={rsdp:#?}");

    println!("retrieving UART config");
    let uart_info = unsafe { find_uart(rsdp) };
    println!("  uart={uart_info:?}");

    println!("exiting boot services");
    let memory_info = exit_boot_services();

    // No (de)allocating or logging beyond this point!
    // We have lost access to the boot services and any attempt to invoke one will panic.

    kernel_tt.load();

    let bootinfo = BootInfo {
        memory: memory_info,
        uart: uart_info,
        acpi_rsdp: PA::new(rsdp as u64),
    };
    kernel_start(&bootinfo);
}

/// Load the kernel binary.
///
/// The kernel binary is expected to be located in the boot file system at `\kernel`, and is
/// expected to be an ELF file. Its loadable segments are read into memory and mapped into the
/// returned page table.
fn load_kernel() -> (fn(&BootInfo) -> !, TranslationTable) {
    let boot_fs = uefi::get_boot_fs();
    let root = boot_fs.open_volume();
    let kernel_file = root.open("\\kernel");

    let mut elf = ElfFile::open(kernel_file);

    let entry = elf.entry();
    let entry = unsafe { mem::transmute::<usize, fn(&BootInfo) -> !>(entry) };

    let alloc_frame = || {
        let buffer = uefi::allocate_page();
        PA::new(buffer.as_mut_ptr() as u64)
    };

    let mut page_table = TranslationTable::new(VA::new(0), alloc_frame);
    let phdrs = elf.program_headers();
    for phdr in phdrs {
        if !phdr.is_load() {
            continue;
        }

        let size = phdr.memory_size() as usize;
        let buffer = uefi::allocate_page_memory(size);
        elf.read_segment(&phdr, buffer);

        let pa = PA::new(buffer.as_ptr() as u64);
        let va = VA::new(phdr.virtual_address());
        let size = buffer.len();
        page_table.map_region(va, pa, size, alloc_frame);
        println!("  mapped {va:#} -> {pa:#} ({size:#x} bytes)");
    }

    (entry, page_table)
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
unsafe fn find_uart(rsdp: *mut acpi::RSDP) -> info::Uart {
    assert_eq!((*rsdp).signature, *b"RSD PTR ");
    assert_eq!((*rsdp).revision, 2);

    let xsdt = (*rsdp).xsdt_address;
    assert_eq!((*xsdt).header.signature, *b"XSDT");
    assert_eq!((*xsdt).header.revision, 1);

    let xsdt_size = (*xsdt).header.length as usize;
    let mut entry_size = xsdt_size - mem::size_of::<acpi::XSDT>();
    let mut entry_ptr = (*xsdt).entry.as_mut_ptr();

    let mut spcr: Option<*mut acpi::SPCR> = None;
    const ADDR_SIZE: usize = mem::size_of::<usize>();
    while entry_size >= ADDR_SIZE {
        let addr_bytes = entry_ptr as *mut [u8; ADDR_SIZE];
        let addr = usize::from_le_bytes(*addr_bytes);
        let desc = addr as *mut acpi::DESCRIPTION_HEADER;
        if (*desc).signature == *b"SPCR" {
            spcr = Some(desc.cast());
            break;
        }

        entry_ptr = entry_ptr.add(ADDR_SIZE);
        entry_size -= ADDR_SIZE;
    }

    let spcr = spcr.expect("SPCR table present");
    assert_eq!((*spcr).header.revision, 2);

    let base = (*spcr).base_address.address;

    match (*spcr).interface_type {
        acpi::UART_TYPE_16550 => info::Uart::Uart16550 { base },
        acpi::UART_TYPE_PL011 => info::Uart::Pl011 { base },
        value => unimplemented!("UART type: {value:#x}"),
    }
}

/// Exit the UEFI boot services.
///
/// Returns information about the physical memory in the system.
fn exit_boot_services() -> info::Memory {
    let (buffer_size, desc_size) = uefi::get_memory_map_size();
    let len = buffer_size / desc_size;

    // Allocating these `Vec`s may add entries to the memory map, so we need to overprovision.
    let buffer = vec![0; buffer_size + 1024];
    let mut block_info = Vec::with_capacity(len + 5);

    let memory_map = uefi::get_memory_map(buffer);

    uefi::exit_boot_services(memory_map.map_key);

    for desc in memory_map.iter() {
        if let Ok(type_) = desc.type_.try_into() {
            let block = info::MemoryBlock {
                type_,
                start: desc.physical_start.into(),
                pages: desc.number_of_pages as usize,
            };
            block_info.push(block);
        }
    }

    // We can't deallocate anymore, so we must avoid dropping the `MemoryMap`.
    mem::forget(memory_map);

    info::Memory::new(block_info)
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
