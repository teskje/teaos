//! UEFI loader for TeaOS.

use core::ffi::c_void;
use core::mem;

use crate::log::{self, println};
use crate::uart::Uart;
use crate::{kernel_main, BootConfig};

use self::api::{Api, MemoryMap};

mod api;
mod sys;

pub use self::api::ConsoleOut;

#[no_mangle]
unsafe extern "efiapi" fn efi_main(
    image_handle: sys::HANDLE,
    system_table: *mut sys::SYSTEM_TABLE,
) -> sys::STATUS {
    let efi = Api::new(image_handle, system_table);

    log::set_uefi(efi.console_out());
    println!("entered UEFI load");

    println!("retrieving image base");
    let image_base = get_image_base(&efi);
    println!("  image_base={image_base:#?}");

    println!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp(&efi);
    println!("  rsdp_ptr={rsdp:#?}");

    println!("retrieving UART config");
    let uart = find_uart(rsdp);
    println!("  uart={uart:?}");

    println!("retrieving memory map");
    let memory_map = efi.get_memory_map();
    dump_memory_map(&memory_map);

    println!("exiting boot services");
    log::set_none();

    efi.exit_boot_services(memory_map.map_key);

    let boot_config = BootConfig {
        rsdp: rsdp.cast(),
        uart,
    };

    kernel_main(boot_config);
}

fn get_image_base(efi: &Api) -> *mut c_void {
    let protocol = efi.loaded_image_protocol();
    protocol.image_base()
}

fn find_acpi_rsdp(efi: &Api) -> *mut acpi::RSDP {
    for (guid, ptr) in efi.config_table().iter() {
        if guid == sys::ACPI_TABLE_GUID {
            return ptr.cast();
        }
    }

    panic!("ACPI config table absent");
}

fn dump_memory_map(memory_map: &MemoryMap) {
    println!("  type    physical_start     virtual_start  num_pages         attribute");
    println!("  ----  ----------------  ----------------  ---------  ----------------");

    for entry in memory_map.iter() {
        println!(
            "  {:>4}  {:016x}  {:016x}  {:>9}  {:016x}",
            entry.type_(),
            entry.physical_start(),
            entry.virtual_start(),
            entry.number_of_pages(),
            entry.attribute(),
        );
    }
}

unsafe fn find_uart(rsdp: *mut acpi::RSDP) -> Uart {
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
        acpi::UART_TYPE_16550 => Uart::uart_16550(uart_base),
        acpi::UART_TYPE_PL011 => Uart::pl011(uart_base),
        value => unimplemented!("UART type: {value:#x}"),
    }
}

/// ACPI type definitions.
#[allow(non_camel_case_types, dead_code)]
mod acpi {
    /// [ACPI] 5.2.3.2 Generic Address Structure
    #[repr(packed)]
    pub struct GAS {
        pub address_space_id: u8,
        pub register_bit_width: u8,
        pub register_bit_offset: u8,
        pub access_size: u8,
        pub address: u64,
    }

    /// [ACPI] 5.2.5.3 Root System Description Pointer (RSDP) Structure
    #[repr(packed)]
    pub struct RSDP {
        pub signature: [u8; 8],
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub revision: u8,
        pub rsdt_address: u32,
        pub length: u32,
        pub xsdt_address: *mut XSDT,
        pub extended_checksum: u8,
        reserved: [u8; 3],
    }

    /// [ACPI] 5.2.6 System Description Table Header
    #[repr(packed)]
    pub struct DESCRIPTION_HEADER {
        pub signature: [u8; 4],
        pub length: u32,
        pub revision: u8,
        pub checksum: u8,
        pub oem_id: [u8; 6],
        pub oem_table_id: [u8; 8],
        pub oem_revision: u32,
        pub creator_id: [u8; 4],
        pub creator_revision: u32,
    }

    /// [APIC] 5.2.8 Extended System Description Table (XSDT)
    #[repr(packed)]
    pub struct XSDT {
        pub header: DESCRIPTION_HEADER,
        pub entry: [u8; 0],
    }

    /// https://learn.microsoft.com/en-us/windows-hardware/drivers/serports/serial-port-console-redirection-table
    #[repr(packed)]
    pub struct SPCR {
        pub header: DESCRIPTION_HEADER,
        pub interface_type: u8,
        reserved: [u8; 3],
        pub base_address: GAS,
        pub interrupt_type: u8,
        pub irq: u8,
        pub global_system_interrupt: u32,
        pub configured_baud_rate: u8,
        pub parity: u8,
        pub stop_bits: u8,
        pub flow_control: u8,
        pub terminal_type: u8,
        pub language: u8,
        pub pci_device: u16,
        pub pci_vendor_id: u16,
        pub pci_bus_number: u8,
        pub pci_device_number: u8,
        pub pci_function_number: u8,
        pub pci_flags: u32,
        pub pci_segment: u8,
        pub uart_clock_frequency: u32,
    }

    /// https://learn.microsoft.com/en-us/windows-hardware/drivers/bringup/acpi-debug-port-table
    pub const UART_TYPE_16550: u8 = 0x00;
    pub const UART_TYPE_PL011: u8 = 0x03;
}
