//! FFI definitions for UEFI types.
//!
//! Extracted from the [UEFI] specification.
//!
//! [UEFI]: https://uefi.org/sites/default/files/resources/UEFI_Spec_2_10_Aug29.pdf

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::c_void;

// 2.3 Calling Conventions
// -----------------------

pub type STATUS = usize;

pub type HANDLE = *mut c_void;

pub type GUID = [u8; 16];

// 4.2 EFI Table Header
// --------------------

#[derive(Debug)]
#[repr(C)]
pub struct TABLE_HEADER {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

// 4.3 EFI System Table
// --------------------

pub const SYSTEM_TABLE_SIGNATURE: u64 = 0x5453595320494249;

#[derive(Debug)]
#[repr(C)]
pub struct SYSTEM_TABLE {
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

// 4.4 EFI Boot Services Table
// ---------------------------

pub const BOOT_SERVICES_SIGNATURE: u64 = 0x56524553544f4f42;

#[derive(Debug)]
#[repr(C)]
pub struct BOOT_SERVICES {
    pub hdr: TABLE_HEADER,
    pub raise_tpl: *mut c_void,
    pub restore_tpl: *mut c_void,
    pub allocate_pages: *mut c_void,
    pub free_pages: *mut c_void,
    pub get_memory_map: GET_MEMORY_MAP,
    pub allocate_pool: ALLOCATE_POOL,
    pub free_pool: FREE_POOL,
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

// 4.6 EFI Configuration Table & Properties Table
// ----------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub struct CONFIGURATION_TABLE {
    pub vendor_guid: GUID,
    pub vendor_table: *mut c_void,
}

pub const ACPI_TABLE_GUID: GUID = [
    0x71, 0xe8, 0x68, 0x88, 0xf1, 0xe4, 0xd3, 0x11, 0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81,
];

// 7.2 Memory Allocation Services
// ------------------------------

pub type PHYSICAL_ADDRESS = u64;
pub type VIRTUAL_ADDRESS = u64;

pub type MEMORY_TYPE = u32;

pub const ReservedMemoryType: MEMORY_TYPE = 0;
pub const LoaderCode: MEMORY_TYPE = 1;
pub const LoaderData: MEMORY_TYPE = 2;
pub const BootServicesCode: MEMORY_TYPE = 3;
pub const BootServicesData: MEMORY_TYPE = 4;
pub const RuntimeServicesCode: MEMORY_TYPE = 5;
pub const RuntimeServicesData: MEMORY_TYPE = 6;
pub const ConventionalMemory: MEMORY_TYPE = 7;
pub const UnusableMemory: MEMORY_TYPE = 8;
pub const ACPIReclaimMemory: MEMORY_TYPE = 9;
pub const ACPIMemoryNVS: MEMORY_TYPE = 10;
pub const MemoryMappedIO: MEMORY_TYPE = 11;
pub const MemoryMappedIOPortSpace: MEMORY_TYPE = 12;
pub const PalCode: MEMORY_TYPE = 13;
pub const PersistentMemory: MEMORY_TYPE = 14;
pub const UnacceptedMemoryType: MEMORY_TYPE = 15;

#[derive(Debug)]
#[repr(C)]
pub struct MEMORY_DESCRIPTOR {
    pub type_: MEMORY_TYPE,
    pub physical_start: PHYSICAL_ADDRESS,
    pub virtual_start: VIRTUAL_ADDRESS,
    pub number_of_pages: u64,
    pub attribute: u64,
}

pub const MEMORY_DESCRIPTOR_VERSION: u32 = 1;

pub type GET_MEMORY_MAP = extern "efiapi" fn(
    memory_map_size: *mut usize,
    memory_map: *mut c_void,
    map_key: *mut usize,
    descriptor_size: *mut usize,
    descriptor_version: *mut u32,
) -> STATUS;

pub type ALLOCATE_POOL =
    extern "efiapi" fn(pool_type: MEMORY_TYPE, size: usize, buffer: *mut *mut c_void) -> STATUS;

pub type FREE_POOL = extern "efiapi" fn(buffer: *mut c_void) -> STATUS;

// 7.3 Protocol Handler Services
// -----------------------------

pub type HANDLE_PROTOCOL = extern "efiapi" fn(
    handle: HANDLE,
    protocol: *const GUID,
    interface: *mut *mut c_void,
) -> STATUS;

// 7.4 Image Services
// ------------------

pub type EXIT_BOOT_SERVICES = extern "efiapi" fn(image_handle: HANDLE, map_key: usize) -> STATUS;

// 9.1 EFI Loaded Image Protocol
// -----------------------------

pub const LOADED_IMAGE_PROTOCOL_GUID: GUID = [
    0xa1, 0x31, 0x1b, 0x5b, 0x62, 0x95, 0xd2, 0x11, 0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b,
];

pub const LOADED_IMAGE_PROTOCOL_REVISION: u32 = 0x1000;

#[derive(Debug)]
#[repr(C)]
pub struct LOADED_IMAGE_PROTOCOL {
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
    pub image_unload: *mut c_void,
}

// 12.4 Simple Text Output Protocol
// --------------------------------

#[derive(Debug)]
#[repr(C)]
pub struct SIMPLE_TEXT_OUTPUT_PROTOCOL {
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

pub type TEXT_STRING =
    extern "efiapi" fn(this: *mut SIMPLE_TEXT_OUTPUT_PROTOCOL, string: *const u16) -> STATUS;

// 13.4 Simple File System Protocol
// --------------------------------

pub const SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: GUID = [
    0x22, 0x5b, 0x4e, 0x96, 0x59, 0x64, 0xd2, 0x11, 0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b,
];

pub const SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION: u64 = 0x10000;

#[derive(Debug)]
#[repr(C)]
pub struct SIMPLE_FILE_SYSTEM_PROTOCOL {
    pub revision: u64,
    pub open_volume: SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME,
}

pub type SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME = extern "efiapi" fn(
    this: *mut SIMPLE_FILE_SYSTEM_PROTOCOL,
    root: *mut *mut FILE_PROTOCOL,
) -> STATUS;

// 13.5 File Protocol
// ------------------

pub const FILE_PROTOCOL_REVISION: u64 = 0x10000;

#[derive(Debug)]
#[repr(C)]
pub struct FILE_PROTOCOL {
    pub revision: u64,
    pub open: FILE_OPEN,
    pub close: FILE_CLOSE,
    pub delete: *mut c_void,
    pub read: *mut c_void,
    pub write: *mut c_void,
    pub get_position: *mut c_void,
    pub set_position: *mut c_void,
    pub get_info: *mut c_void,
    pub set_info: *mut c_void,
    pub flush: *mut c_void,
}

pub const FILE_MODE_READ: u64 = 0x0000000000000001;

pub type FILE_OPEN = extern "efiapi" fn(
    this: *mut FILE_PROTOCOL,
    new_handle: *mut *mut FILE_PROTOCOL,
    file_name: *const u16,
    open_mode: u64,
    attributes: u64,
) -> STATUS;

pub type FILE_CLOSE = extern "efiapi" fn(this: *mut FILE_PROTOCOL) -> STATUS;

// Appendix D

pub const SUCCESS: STATUS = 0;
pub const BUFFER_TOO_SMALL: STATUS = (1 << 63) | 5;
