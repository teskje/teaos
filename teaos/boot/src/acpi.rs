//! FFI definitions for ACPI types.
//!
//! Extracted from the [ACPI] specification.
//!
//! [ACPI]: https://uefi.org/sites/default/files/resources/ACPI_Spec_6_5_Aug29.pdf

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(clippy::upper_case_acronyms)]

// 5.2 ACPI System Description Tables
// ----------------------------------

use aarch64::memory::PA;

#[repr(C, packed)]
pub struct GAS {
    pub address_space_id: u8,
    pub register_bit_width: u8,
    pub register_bit_offset: u8,
    pub access_size: u8,
    pub address: PA,
}

#[repr(C, packed)]
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

#[repr(C, packed)]
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

#[repr(C, packed)]
pub struct XSDT {
    pub header: DESCRIPTION_HEADER,
    pub entry: [u8; 0],
}

// learn.microsoft.com
// -------------------

#[repr(C, packed)]
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

pub const UART_TYPE_16550: u8 = 0x00;
pub const UART_TYPE_PL011: u8 = 0x03;
