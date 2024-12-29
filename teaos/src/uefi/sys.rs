//! FFI definitions for UEFI types.
//!
//! Extracted from the [UEFI] specification.
//!
//! [UEFI]: https://uefi.org/sites/default/files/resources/UEFI_Spec_2_10_Aug29.pdf

#![allow(non_camel_case_types)]

use core::ffi::c_void;

/// [UEFI] 2.3.1 Data Types
/// [UEFI] Appendix D Status Codes
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum STATUS {
    SUCCESS = 0,
    BUFFER_TOO_SMALL = (1 << 63) | 5,
}

/// [UEFI] 2.3.1 Data Types
pub(super) type HANDLE = *mut c_void;

/// [UEFI] 2.3.1 Data Types
pub(super) type GUID = [u8; 16];

/// [UEFI] 4.2.1 EFI_TABLE_HEADER
#[repr(C)]
#[derive(Debug)]
pub(super) struct TABLE_HEADER {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

/// [UEFI] 4.3.1 EFI_SYSTEM_TABLE
pub(super) const SYSTEM_TABLE_SIGNATURE: u64 = 0x5453595320494249;

/// [UEFI] 4.3.1 EFI_SYSTEM_TABLE
#[repr(C)]
#[derive(Debug)]
pub(super) struct SYSTEM_TABLE {
    pub hdr: TABLE_HEADER,
    pub firmware_vendor: *mut c_void,
    pub firmware_revision: u32,
    pub console_in_handle: HANDLE,
    pub con_in: *mut c_void,
    pub console_out_handle: HANDLE,
    pub con_out: *mut SIMPLE_TEXT_OUTPUT_PROTOCOL,
    pub standard_error_handle: HANDLE,
    pub std_err: *mut c_void,
    pub runtime_services: *mut c_void,
    pub boot_services: *mut BOOT_SERVICES,
    pub number_of_table_entries: usize,
    pub configuration_table: *mut CONFIGURATION_TABLE,
}

/// [UEFI] 4.4.1 EFI_BOOT_SERVICES
pub(super) const BOOT_SERVICES_SIGNATURE: u64 = 0x56524553544f4f42;

/// [UEFI] 4.4.1 EFI_BOOT_SERVICES
#[repr(C)]
#[derive(Debug)]
pub(super) struct BOOT_SERVICES {
    pub hdr: TABLE_HEADER,
    pub raise_tpl: *mut c_void,
    pub restore_tpl: *mut c_void,
    pub allocate_pages: *mut c_void,
    pub free_pages: *mut c_void,
    pub get_memory_map: GET_MEMORY_MAP,
    pub allocate_pool: ALLOCATE_POOL,
    pub free_pool: *mut c_void,
    pub create_event: *mut c_void,
    pub set_timer: *mut c_void,
    pub wait_for_event: *mut c_void,
    pub signal_event: *mut c_void,
    pub close_event: *mut c_void,
    pub check_event: *mut c_void,
    pub install_protocol_interface: *mut c_void,
    pub reinstall_protocol_interface: *mut c_void,
    pub uninstall_protocol_interface: *mut c_void,
    pub handle_protocol: HANDLE_PROTOCOL,
    pub reserved: *mut c_void,
    pub register_protocol_notify: *mut c_void,
    pub locate_handle: *mut c_void,
    pub locate_device_path: *mut c_void,
    pub install_configuration_table: *mut c_void,
    pub load_image: *mut c_void,
    pub start_image: *mut c_void,
    pub exit: *mut c_void,
    pub unload_image: *mut c_void,
    pub exit_boot_services: EXIT_BOOT_SERVICES,
    pub get_next_monotonic_count: *mut c_void,
    pub stall: *mut c_void,
    pub set_watchdog_timer: *mut c_void,
    pub connect_controller: *mut c_void,
    pub disconnect_controller: *mut c_void,
    pub open_protocol: *mut c_void,
    pub close_protocol: *mut c_void,
    pub open_protocol_information: *mut c_void,
    pub protocols_per_handle: *mut c_void,
    pub locate_handle_buffer: *mut c_void,
    pub locate_protocol: *mut c_void,
    pub install_multiple_protocol_interfaces: *mut c_void,
    pub uninstall_multiple_protocol_interfaces: *mut c_void,
    pub calculate_crc32: *mut c_void,
    pub copy_mem: *mut c_void,
    pub set_mem: *mut c_void,
    pub create_event_ex: *mut c_void,
}

/// [UEFI] 4.6.1 EFI_CONFIGURATION_TABLE
#[derive(Debug)]
pub(super) struct CONFIGURATION_TABLE {
    pub vendor_guid: GUID,
    pub vendor_table: *mut c_void,
}

/// [UEFI] 4.6.1.1 Industry Standard Configuration Tables
pub(super) const ACPI_TABLE_GUID: GUID = [
    0x71, 0xe8, 0x68, 0x88, 0xf1, 0xe4, 0xd3, 0x11, 0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81,
];

/// [UEFI] 7.2.1 EFI_BOOT_SERVICES.AllocatePages()
#[repr(u32)]
#[derive(Debug)]
pub(super) enum MEMORY_TYPE {
    LoaderData = 2,
}

/// [UEFI] 7.2.1 EFI_BOOT_SERVICES.AllocatePages()
pub(super) type PHYSICAL_ADDRESS = u64;

/// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
pub(super) type GET_MEMORY_MAP =
    extern "efiapi" fn(*mut usize, *mut c_void, *mut usize, *mut usize, *mut u32) -> STATUS;

/// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
#[repr(C)]
#[derive(Debug)]
pub(super) struct MEMORY_DESCRIPTOR {
    pub type_: u32,
    pub physical_start: PHYSICAL_ADDRESS,
    pub virtual_start: VIRTUAL_ADDRESS,
    pub number_of_pages: u64,
    pub attribute: u64,
}

/// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
pub(super) type VIRTUAL_ADDRESS = u64;

/// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
pub(super) const MEMORY_DESCRIPTOR_VERSION: u32 = 1;

/// [UEFI] 7.2.4 EFI_BOOT_SERVICES.AllocatePool()
pub(super) type ALLOCATE_POOL = extern "efiapi" fn(MEMORY_TYPE, usize, *mut *mut c_void) -> STATUS;

/// [UEFI] 7.3.7 EFI_BOOT_SERVICES.HandleProtocol()
pub(super) type HANDLE_PROTOCOL = extern "efiapi" fn(
    handle: HANDLE,
    protocol: *const GUID,
    interface: *mut *mut c_void,
) -> STATUS;

/// [UEFI] 7.4.6 EFI_BOOT_SERVICES.ExitBootServices()
pub(super) type EXIT_BOOT_SERVICES = extern "efiapi" fn(HANDLE, usize) -> STATUS;

/// [UEFI] 9.1.1 EFI_LOADED_IMAGE_PROTOCOL
pub(super) const LOADED_IMAGE_PROTOCOL_GUID: GUID = [
    0xA1, 0x31, 0x1B, 0x5B, 0x62, 0x95, 0xd2, 0x11, 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B,
];

/// [UEFI] 9.1.1 EFI_LOADED_IMAGE_PROTOCOL
pub(super) const LOADED_IMAGE_PROTOCOL_REVISION: u32 = 0x1000;

/// [UEFI] 9.1.1 EFI_LOADED_IMAGE_PROTOCOL
#[repr(C)]
#[derive(Debug)]
pub(super) struct LOADED_IMAGE_PROTOCOL {
    pub revision: u32,
    pub parent_handle: HANDLE,
    pub system_table: *mut SYSTEM_TABLE,
    pub device_handle: HANDLE,
    pub file_path: *mut c_void,
    pub reserved: *mut c_void,
    pub load_options_size: u32,
    pub load_options: *mut c_void,
    pub image_base: *mut c_void,
    pub image_size: u64,
    pub image_code_type: MEMORY_TYPE,
    pub image_data_type: MEMORY_TYPE,
    pub unload: *mut c_void,
}

/// [UEFI] 12.4.1 EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
#[repr(C)]
#[derive(Debug)]
pub(super) struct SIMPLE_TEXT_OUTPUT_PROTOCOL {
    pub reset: *mut c_void,
    pub output_string: TEXT_STRING,
    pub test_string: *mut c_void,
    pub query_mode: *mut c_void,
    pub set_mode: *mut c_void,
    pub set_attribute: *mut c_void,
    pub clear_screen: *mut c_void,
    pub set_cursor_position: *mut c_void,
    pub enable_cursor: *mut c_void,
    pub mode: *mut c_void,
}

/// [UEFI] 12.4.3 EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL.OutputString()
type TEXT_STRING = extern "efiapi" fn(*mut SIMPLE_TEXT_OUTPUT_PROTOCOL, *const u16) -> STATUS;