#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod info;
pub mod log;

mod acpi;
mod allocator;
mod crc32;
mod sync;
mod uefi;

use core::ffi::c_void;
use core::mem;

/// # Safety
///
/// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
pub unsafe fn init_uefi(image_handle: *mut c_void, system_table: *mut c_void) {
    uefi::init(image_handle, system_table.cast());
}

pub fn load() -> ! {
    println!("entered UEFI boot loader");

    println!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp();
    println!("  rsdp_ptr={rsdp:#?}");

    println!("retrieving UART config");
    let uart = unsafe { find_uart(rsdp) };
    println!("  uart={uart:?}");

    println!("opening kernel file");
    let boot_fs = uefi::get_boot_fs();
    let root = boot_fs.open_volume();
    let kernel_file = root.open("\\kernel");
    drop((boot_fs, root, kernel_file));

    println!("retrieving memory map");
    let memory_map = uefi::get_memory_map();
    dump_memory_map(&memory_map);

    println!("exiting boot services");
    uefi::exit_boot_services(memory_map.map_key);

    loop {}

    //let boot_config = BootConfig {
    //    rsdp: rsdp.cast(),
    //    uart,
    //};
    //
    //kernel_main(boot_config);
}

fn dump_memory_map(memory_map: &uefi::MemoryMap) {
    println!("  type    physical_start     virtual_start  num_pages         attribute");
    println!("  ----  ----------------  ----------------  ---------  ----------------");

    for entry in memory_map.iter() {
        println!(
            "  {:>4}  {:016x}  {:016x}  {:>9}  {:016x}",
            entry.type_,
            entry.physical_start,
            entry.virtual_start,
            entry.number_of_pages,
            entry.attribute,
        );
    }
}

fn find_acpi_rsdp() -> *mut acpi::RSDP {
    for (guid, ptr) in uefi::config_table().iter() {
        if guid == uefi::sys::ACPI_TABLE_GUID {
            return ptr.cast();
        }
    }

    panic!("ACPI config table not found");
}

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

    let uart_base = (*spcr).base_address.address;
    let uart_base = uart_base as *mut c_void;

    match (*spcr).interface_type {
        acpi::UART_TYPE_16550 => info::Uart::Uart16550 { base: uart_base },
        acpi::UART_TYPE_PL011 => info::Uart::Pl011 { base: uart_base },
        value => unimplemented!("UART type: {value:#x}"),
    }
}
