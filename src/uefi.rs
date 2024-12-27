//! UEFI loader for TeaOS.

use core::ffi::c_void;
use core::{fmt, mem, ptr};

use crate::log::{self, println};
use crate::uart::Uart;
use crate::{kernel_main, BootConfig};

#[no_mangle]
unsafe extern "efiapi" fn efi_main(
    image_handle: efi::HANDLE,
    system_table: *mut efi::SYSTEM_TABLE,
) -> efi::STATUS {
    let bs = (*system_table).boot_services;
    let con_out = (*system_table).con_out;

    log::set_uefi(ConOut(con_out));
    println!("entered UEFI load");

    println!("retrieving image base");
    let image_base = get_image_base(bs, image_handle);
    println!("  image_base={image_base:#?}");

    println!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp(system_table);
    println!("  rsdp_ptr={rsdp:#?}");

    println!("retrieving UART config");
    let uart = find_uart(rsdp);
    println!("  uart={uart:?}");

    println!("retrieving memory map");
    let memory_map = get_memory_map(bs);
    dump_memory_map(&memory_map);

    println!("exiting boot services");
    log::set_none();

    exit_boot_services(bs, image_handle, memory_map.map_key);

    let boot_config = BootConfig {
        rsdp: rsdp.cast(),
        uart,
    };

    kernel_main(boot_config);
}

unsafe fn get_image_base(bs: *mut efi::BOOT_SERVICES, image_handle: efi::HANDLE) -> *mut c_void {
    let mut loaded_image = ptr::null_mut();
    let handle_protocol = (*bs).handle_protocol;
    let status = handle_protocol(
        image_handle,
        &efi::LOADED_IMAGE_PROTOCOL_GUID,
        &mut loaded_image,
    );
    assert_eq!(status, efi::STATUS::SUCCESS);

    let loaded_image = loaded_image as *mut efi::LOADED_IMAGE_PROTOCOL;
    assert_eq!((*loaded_image).revision, 0x1000);
    (*loaded_image).image_base
}

unsafe fn find_acpi_rsdp(system_table: *mut efi::SYSTEM_TABLE) -> *mut acpi::RSDP {
    let cfg_table = (*system_table).configuration_table;
    let cfg_table_len = (*system_table).number_of_table_entries;

    let mut rsdp_ptr = None;
    for idx in 0..cfg_table_len {
        let entry = cfg_table.add(idx);
        if (*entry).vendor_guid == efi::ACPI_TABLE_GUID {
            rsdp_ptr = Some((*entry).vendor_table);
            break;
        }
    }

    let rsdp_ptr = rsdp_ptr.expect("ACPI table present");
    rsdp_ptr.cast()
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

    let uart_base = acpi_gas_address((*spcr).base_address);
    let uart_base = uart_base as *mut c_void;

    match (*spcr).interface_type {
        acpi::UART_TYPE_16550 => Uart::uart_16550(uart_base),
        acpi::UART_TYPE_PL011 => Uart::pl011(uart_base),
        value => unimplemented!("UART type: {value:#x}"),
    }
}

fn acpi_gas_address(gas: acpi::GAS) -> u64 {
    let address_bytes = gas[4..].try_into().unwrap();
    u64::from_le_bytes(address_bytes)
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

    /// [UEFI] 7.3.7 EFI_BOOT_SERVICES.HandleProtocol()
    pub type HANDLE_PROTOCOL = extern "efiapi" fn(
        handle: HANDLE,
        protocol: *const GUID,
        interface: *mut *mut c_void,
    ) -> STATUS;

    /// [UEFI] 7.4.6 EFI_BOOT_SERVICES.ExitBootServices()
    pub type EXIT_BOOT_SERVICES = extern "efiapi" fn(HANDLE, usize) -> STATUS;

    /// [UEFI] 9.1.1 EFI_LOADED_IMAGE_PROTOCOL
    pub const LOADED_IMAGE_PROTOCOL_GUID: GUID = [
        0xA1, 0x31, 0x1B, 0x5B, 0x62, 0x95, 0xd2, 0x11, 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72,
        0x3B,
    ];

    /// [UEFI] 9.1.1 EFI_LOADED_IMAGE_PROTOCOL
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
        pub unload: *mut c_void,
    }

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

/// ACPI type definitions.
#[allow(non_camel_case_types)]
mod acpi {
    /// [ACPI] 5.2.3.2 Generic Address Structure
    pub type GAS = [u8; 12];

    /// [ACPI] 5.2.5.3 Root System Description Pointer (RSDP) Structure
    #[repr(C)]
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
    #[repr(C)]
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
    #[repr(C)]
    pub struct XSDT {
        pub header: DESCRIPTION_HEADER,
        pub entry: [u8; 0],
    }

    /// https://learn.microsoft.com/en-us/windows-hardware/drivers/serports/serial-port-console-redirection-table
    ///
    /// NOTE: Some `u32`s in this struct are not properly 4-byte aligned, so we specify them as
    /// `[u8; 4]` instead.
    #[repr(C)]
    pub struct SPCR {
        pub header: DESCRIPTION_HEADER,
        pub interface_type: u8,
        reserved: [u8; 3],
        pub base_address: GAS,
        pub interrupt_type: u8,
        pub irq: u8,
        pub global_system_interrupt: [u8; 4],
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
        pub pci_flags: [u8; 4],
        pub pci_segment: u8,
        pub uart_clock_frequency: [u8; 4],
    }

    /// https://learn.microsoft.com/en-us/windows-hardware/drivers/bringup/acpi-debug-port-table
    pub const UART_TYPE_16550: u8 = 0x00;
    pub const UART_TYPE_PL011: u8 = 0x03;
}
