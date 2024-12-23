//! UEFI loader for TeaOS.

use core::ffi::c_void;
use core::{fmt, mem, ptr};

use crate::kernel_main;
use crate::log::{self, println};

#[no_mangle]
unsafe extern "efiapi" fn efi_main(
    image_handle: efi::HANDLE,
    system_table: *mut efi::SYSTEM_TABLE,
) -> efi::STATUS {
    let bs = (*system_table).boot_services;
    let con_out = (*system_table).con_out;

    log::set_uefi(ConOut(con_out));

    println!("commencing UEFI load");

    println!("retrieving ACPI RSDP pointer");
    let rsdp_ptr = find_rsdp_ptr(system_table);
    println!("  rsdp_ptr={rsdp_ptr:#?}");

    // TODO retrieve the UART config

    println!("retrieving memory map");
    let memory_map = get_memory_map(bs);
    dump_memory_map(&memory_map);

    println!("exiting boot services");
    exit_boot_services(bs, image_handle, memory_map.map_key);

    kernel_main(rsdp_ptr);
}

unsafe fn find_rsdp_ptr(system_table: *mut efi::SYSTEM_TABLE) -> *mut c_void {
    let cfg_table = (*system_table).configuration_table;
    let cfg_table_len = (*system_table).number_of_table_entries;

    let mut rsdp = None;
    for idx in 0..cfg_table_len {
        let entry = cfg_table.add(idx);
        if (*entry).vendor_guid == efi::ACPI_TABLE_GUID {
            rsdp = Some((*entry).vendor_table);
            break;
        }
    }

    rsdp.expect("ACPI table present")
}

unsafe fn exit_boot_services(
    bs: *mut efi::BOOT_SERVICES,
    image_handle: efi::HANDLE,
    map_key: usize,
) {
    let exit_boot_services = (*bs).exit_boot_services;
    let status = exit_boot_services(image_handle, map_key);
    assert_eq!(status, efi::STATUS::SUCCESS);
}

unsafe fn get_memory_map(bs: *mut efi::BOOT_SERVICES) -> MemoryMap {
    let get_memory_map = (*bs).get_memory_map;

    let mut empty_buffer = [];
    let mut memory_map = MemoryMap {
        buffer: empty_buffer.as_mut_ptr(),
        buffer_size: 0,
        descriptor_size: 0,
        map_key: 0,
    };
    let mut descriptor_version = 0;

    // Query the required buffer size.
    let status = get_memory_map(
        &mut memory_map.buffer_size,
        memory_map.buffer,
        &mut memory_map.map_key,
        &mut memory_map.descriptor_size,
        &mut descriptor_version,
    );
    assert_eq!(status, efi::STATUS::BUFFER_TOO_SMALL);

    // Allocate a sufficiently large buffer.
    //
    // "The actual size of the buffer allocated for the consequent call to `GetMemoryMap()` should
    // be bigger then the value returned in `MemoryMapSize`, since allocation of the new buffer may
    // potentially increase memory map size."
    memory_map.buffer_size += 1024;
    memory_map.buffer = allocate_pool(bs, memory_map.buffer_size).cast();

    // Get the memory map.
    let status = get_memory_map(
        &mut memory_map.buffer_size,
        memory_map.buffer,
        &mut memory_map.map_key,
        &mut memory_map.descriptor_size,
        &mut descriptor_version,
    );
    assert_eq!(status, efi::STATUS::SUCCESS);
    assert_eq!(descriptor_version, 1);
    assert!(memory_map.descriptor_size >= mem::size_of::<efi::MEMORY_DESCRIPTOR>());

    memory_map
}

unsafe fn allocate_pool(bs: *mut efi::BOOT_SERVICES, size: usize) -> *mut c_void {
    let mut buffer = ptr::null_mut();

    let allocate_pool = (*bs).allocate_pool;
    let status = allocate_pool(efi::MEMORY_TYPE::LoaderData, size, &mut buffer);
    assert_eq!(status, efi::STATUS::SUCCESS);

    buffer
}

fn dump_memory_map(memory_map: &MemoryMap) {
    println!("  type    physical_start     virtual_start  num_pages         attribute");
    println!("  ----  ----------------  ----------------  ---------  ----------------");

    for entry in memory_map.iter() {
        println!(
            "  {:>4}  {:016x}  {:016x}  {:>9}  {:016x}",
            entry.type_,
            entry.physical_start,
            entry.virtual_start,
            entry.number_of_pages,
            entry.attribute
        );
    }
}

pub struct ConOut(*mut efi::SIMPLE_TEXT_OUTPUT_PROTOCOL);

impl fmt::Write for ConOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let output_string = unsafe { (*self.0).output_string };
        for c in s.encode_utf16() {
            let s = [c, 0x0000];
            let status = output_string(self.0, s.as_ptr());

            if status != efi::STATUS::SUCCESS {
                return Err(fmt::Error);
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct MemoryMap {
    buffer: *mut u8,
    buffer_size: usize,
    descriptor_size: usize,
    map_key: usize,
}

impl MemoryMap {
    fn iter(&self) -> impl Iterator<Item = &efi::MEMORY_DESCRIPTOR> {
        let len = self.buffer_size / self.descriptor_size;
        (0..len).into_iter().map(|i| {
            let offset = i * self.descriptor_size;
            unsafe {
                let ptr = self.buffer.add(offset);
                &*(ptr.cast())
            }
        })
    }
}

/// FFI type definitions.
#[allow(non_camel_case_types)]
mod efi {
    use core::ffi::c_void;

    /// [UEFI] 2.3.1 Data Types
    /// [UEFI] Appendix D Status Codes
    #[repr(usize)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum STATUS {
        SUCCESS = 0,
        BUFFER_TOO_SMALL = (1 << 63) | 5,
    }

    /// [UEFI] 2.3.1 Data Types
    pub type HANDLE = *mut c_void;

    /// [UEFI] 2.3.1 Data Types
    pub type GUID = [u8; 16];

    /// [UEFI] 4.2.1 EFI_TABLE_HEADER
    #[repr(C)]
    #[derive(Debug)]
    pub struct TABLE_HEADER {
        pub signature: u64,
        pub revision: u32,
        pub header_size: u32,
        pub crc32: u32,
        pub reserved: u32,
    }

    /// [UEFI] 4.3.1 EFI_SYSTEM_TABLE
    #[repr(C)]
    #[derive(Debug)]
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

    /// [UEFI] 4.4.1 EFI_BOOT_SERVICES
    #[repr(C)]
    #[derive(Debug)]
    pub struct BOOT_SERVICES {
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
        pub handle_protocol: *mut c_void,
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
    pub struct CONFIGURATION_TABLE {
        pub vendor_guid: GUID,
        pub vendor_table: *mut c_void,
    }

    /// [UEFI] 4.6.1.1 Industry Standard Configuration Tables
    pub const ACPI_TABLE_GUID: GUID = [
        0x71, 0xe8, 0x68, 0x88, 0xf1, 0xe4, 0xd3, 0x11, 0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88,
        0x81,
    ];

    /// [UEFI] 7.2.1 EFI_BOOT_SERVICES.AllocatePages()
    #[repr(u32)]
    #[derive(Debug)]
    pub enum MEMORY_TYPE {
        LoaderData = 2,
    }

    /// [UEFI] 7.2.1 EFI_BOOT_SERVICES.AllocatePages()
    pub type PHYSICAL_ADDRESS = u64;

    /// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
    pub type GET_MEMORY_MAP =
        extern "efiapi" fn(*mut usize, *mut u8, *mut usize, *mut usize, *mut u32) -> STATUS;

    /// [UEFI] 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
    #[repr(C)]
    #[derive(Debug)]
    pub struct MEMORY_DESCRIPTOR {
        pub type_: u32,
        pub physical_start: PHYSICAL_ADDRESS,
        pub virtual_start: VIRTUAL_ADDRESS,
        pub number_of_pages: u64,
        pub attribute: u64,
    }

    /// [UEFI} 7.2.3 EFI_BOOT_SERVICES.GetMemoryMap()
    pub type VIRTUAL_ADDRESS = u64;

    /// [UEFI] 7.2.4 EFI_BOOT_SERVICES.AllocatePool()
    pub type ALLOCATE_POOL = extern "efiapi" fn(MEMORY_TYPE, usize, *mut *mut c_void) -> STATUS;

    /// [UEFI] 7.4.6 EFI_BOOT_SERVICES.ExitBootServices()
    pub type EXIT_BOOT_SERVICES = extern "efiapi" fn(HANDLE, usize) -> STATUS;

    /// [UEFI] 12.4.1 EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
    #[repr(C)]
    #[derive(Debug)]
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

    /// {UEFI] 12.4.3 EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL.OutputString()
    type TEXT_STRING = extern "efiapi" fn(*mut SIMPLE_TEXT_OUTPUT_PROTOCOL, *const u16) -> STATUS;
}
