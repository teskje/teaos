//! A safe UEFI API wrapper.

pub mod sys;

use core::ffi::c_void;
use core::{fmt, mem, ptr};

use crate::crc32::Crc32;

pub struct Uefi {
    image_handle: sys::HANDLE,
    system_table: SystemTable,
    boot_services: BootServices,
}

impl Uefi {
    /// # Safety
    ///
    /// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
    pub unsafe fn new(image_handle: sys::HANDLE, system_table: *mut c_void) -> Self {
        let system_table = SystemTable::new(system_table.cast());
        let boot_services = system_table.boot_services();

        Self {
            image_handle,
            system_table,
            boot_services,
        }
    }

    pub fn console_out(&self) -> ConsoleOut {
        self.system_table.console_out()
    }

    pub fn config_table(&self) -> ConfigTable {
        self.system_table.config_table()
    }

    pub fn get_memory_map(&self) -> MemoryMap {
        self.boot_services.get_memory_map()
    }

    /// # Safety
    ///
    /// Calling this method invalidates any references to the boot services and protocols. Callers
    /// must ensure that all such references have been dropped or are otherwise not used anymore.
    pub unsafe fn exit_boot_services(self, map_key: usize) {
        self.boot_services
            .exit_boot_services(self.image_handle, map_key);
    }
}

struct SystemTable {
    ptr: *mut sys::SYSTEM_TABLE,
}

impl SystemTable {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
    unsafe fn new(ptr: *mut sys::SYSTEM_TABLE) -> Self {
        validate_ptr(ptr);
        validate_table_header(&raw const (*ptr).hdr, sys::SYSTEM_TABLE_SIGNATURE);

        Self { ptr }
    }

    fn console_out(&self) -> ConsoleOut {
        // Safety: `self.ptr` is a valid pointer to a `sys::SYSTEM_TABLE`.
        unsafe {
            let ptr = (*self.ptr).con_out;
            ConsoleOut::new(ptr)
        }
    }

    fn boot_services(&self) -> BootServices {
        // Safety: `self.ptr` is a valid pointer to a `sys::SYSTEM_TABLE`.
        unsafe {
            let ptr = (*self.ptr).boot_services;
            BootServices::new(ptr)
        }
    }

    fn config_table(&self) -> ConfigTable {
        // Safety: `self.ptr` is a valid pointer to a `efi::SYSTEM_TABLE`.
        unsafe {
            let ptr = (*self.ptr).configuration_table;
            let len = (*self.ptr).number_of_table_entries;
            ConfigTable::new(ptr, len)
        }
    }
}

pub struct ConsoleOut {
    ptr: *mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL,
}

impl ConsoleOut {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::SIMPLE_TEXT_OUTPUT_PROTOCOL`].
    unsafe fn new(ptr: *mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL) -> Self {
        validate_ptr(ptr);

        Self { ptr }
    }
}

impl fmt::Write for ConsoleOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let output_string = unsafe { (*self.ptr).output_string };
        for c in s.encode_utf16() {
            let s = [c, 0x0000];
            let status = output_string(self.ptr, s.as_ptr());

            if status != sys::STATUS::SUCCESS {
                return Err(fmt::Error);
            }
        }

        Ok(())
    }
}

pub struct BootServices {
    ptr: *mut sys::BOOT_SERVICES,
}

impl BootServices {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::BOOT_SERVICES`].
    unsafe fn new(ptr: *mut sys::BOOT_SERVICES) -> Self {
        validate_ptr(ptr);
        validate_table_header(&raw const (*ptr).hdr, sys::BOOT_SERVICES_SIGNATURE);

        Self { ptr }
    }

    pub fn get_memory_map(&self) -> MemoryMap {
        // Safety: `self.ptr` is a valid pointer to a `sys::BOOT_SERVICES`.
        let get_memory_map = unsafe { (*self.ptr).get_memory_map };

        let mut buffer = [];
        let mut buffer_size = 0;
        let mut map_key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        // Query the required buffer size.
        let status = get_memory_map(
            &mut buffer_size,
            buffer.as_mut_ptr(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        assert_eq!(status, sys::STATUS::BUFFER_TOO_SMALL);

        // Allocate a sufficiently large buffer.
        //
        // "The actual size of the buffer allocated for the consequent call to `GetMemoryMap()`
        // should be bigger then the value returned in `MemoryMapSize`, since allocation of the new
        // buffer may potentially increase memory map size."
        buffer_size += 1024;
        let buffer = self.allocate_pool(buffer_size).cast();

        // Get the memory map.
        let status = get_memory_map(
            &mut buffer_size,
            buffer,
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        assert_eq!(status, sys::STATUS::SUCCESS);
        assert_eq!(descriptor_version, sys::MEMORY_DESCRIPTOR_VERSION);

        // Safety: `get_memory_map` returns correct pointer and size values on `SUCCESS`
        unsafe { MemoryMap::new(buffer, buffer_size, descriptor_size, map_key) }
    }

    fn allocate_pool(&self, size: usize) -> *mut c_void {
        // Safety: `self.ptr` is a valid pointer to a `sys::BOOT_SERVICES`.
        let allocate_pool = unsafe { (*self.ptr).allocate_pool };

        let mut buffer = ptr::null_mut();
        let status = allocate_pool(sys::MEMORY_TYPE::LoaderData, size, &mut buffer);
        assert_eq!(status, sys::STATUS::SUCCESS);

        buffer
    }

    /// # Safety
    ///
    /// Calling this method invalidates any references to the boot services and protocols. Callers
    /// must ensure that all such references have been dropped or are otherwise not used anymore.
    unsafe fn exit_boot_services(self, image_handle: sys::HANDLE, map_key: usize) {
        // Safety: `self.ptr` is a valid pointer to a `sys::BOOT_SERVICES`.
        let exit_boot_services = unsafe { (*self.ptr).exit_boot_services };
        let status = exit_boot_services(image_handle, map_key);
        assert_eq!(status, sys::STATUS::SUCCESS);
    }
}

pub struct ConfigTable {
    ptr: *mut sys::CONFIGURATION_TABLE,
    len: usize,
}

impl ConfigTable {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to an array of `len` [`sys::CONFIGURATION_TABLE`] instances.
    unsafe fn new(ptr: *mut sys::CONFIGURATION_TABLE, len: usize) -> Self {
        validate_ptr(ptr);

        Self { ptr, len }
    }

    pub fn iter(&self) -> impl Iterator<Item = (sys::GUID, *mut c_void)> + '_ {
        (0..self.len).into_iter().map(|i| {
            // Safety: `self.ptr` is a valid pointer to an array of `self.len`
            // `sys::CONFIGURATION_TABLE` instances.
            unsafe {
                let ptr = self.ptr.add(i);
                ((*ptr).vendor_guid, (*ptr).vendor_table)
            }
        })
    }
}

#[derive(Debug)]
pub struct MemoryMap {
    ptr: *mut c_void,
    len: usize,
    descriptor_size: usize,
    pub map_key: usize,
}

impl MemoryMap {
    /// # Safety
    ///
    /// `buffer` must be a valid pointer to an array of `buffer_size / descriptor_size`
    /// [`sys::MEMORY_DESCRIPTOR`]s, each of which is padded according to `descriptor_size`.
    unsafe fn new(
        buffer: *mut c_void,
        buffer_size: usize,
        descriptor_size: usize,
        map_key: usize,
    ) -> Self {
        validate_ptr(buffer);
        assert!(descriptor_size > mem::size_of::<sys::MEMORY_DESCRIPTOR>());

        Self {
            ptr: buffer,
            len: buffer_size / descriptor_size,
            descriptor_size,
            map_key,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = MemoryDescriptor> + '_ {
        (0..self.len).into_iter().map(|i| {
            let offset = i * self.descriptor_size;
            // Safety: `self.ptr` is a valid pointer to an array of `self.len`
            // `sys::MEMORY_DESCRIPTOR` instances.
            unsafe {
                let ptr = self.ptr.add(offset).cast();
                MemoryDescriptor::new(ptr)
            }
        })
    }
}

pub struct MemoryDescriptor {
    ptr: *mut sys::MEMORY_DESCRIPTOR,
}

impl MemoryDescriptor {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::MEMORY_DESCRIPTOR`].
    unsafe fn new(ptr: *mut sys::MEMORY_DESCRIPTOR) -> Self {
        validate_ptr(ptr);

        Self { ptr }
    }

    pub fn type_(&self) -> u32 {
        // Safety: `self.ptr` is a valid pointer to a `sys::MEMORY_DESCRIPTOR`
        unsafe { (*self.ptr).type_ }
    }

    pub fn physical_start(&self) -> sys::PHYSICAL_ADDRESS {
        // Safety: `self.ptr` is a valid pointer to a `sys::MEMORY_DESCRIPTOR`
        unsafe { (*self.ptr).physical_start }
    }

    pub fn virtual_start(&self) -> sys::VIRTUAL_ADDRESS {
        // Safety: `self.ptr` is a valid pointer to a `sys::MEMORY_DESCRIPTOR`
        unsafe { (*self.ptr).physical_start }
    }

    pub fn number_of_pages(&self) -> u64 {
        // Safety: `self.ptr` is a valid pointer to a `sys::MEMORY_DESCRIPTOR`
        unsafe { (*self.ptr).number_of_pages }
    }

    pub fn attribute(&self) -> u64 {
        // Safety: `self.ptr` is a valid pointer to a `sys::MEMORY_DESCRIPTOR`
        unsafe { (*self.ptr).attribute }
    }
}

/// Validate the given pointer.
///   
/// # Panics
///
/// Panics if the given pointer is NULL.
/// Panics if the given pointer is not correctly aligned.
fn validate_ptr<T>(ptr: *mut T) {
    assert!(!ptr.is_null());
    assert!(ptr.is_aligned());
}

/// Validate the table header referenced by the given pointer.
///
/// # Panics
///
/// Panics if the header doesn't have the expected signature.
/// Panics if the header doesn't have the expected revision.
/// Panics if the header's checksum doesn't match.
///
/// # Safety
///
/// `ptr` must be a valid pointer to a [`sys::TABLE_HEADER`], as well as `header_size` subsequent
/// bytes.
unsafe fn validate_table_header(ptr: *const sys::TABLE_HEADER, signature: u64) {
    assert_eq!((*ptr).signature, signature);
    assert_eq!((*ptr).revision & (2 << 16), 2 << 16);

    let start: *const u8 = ptr.cast();
    let crc32_start: *const u8 = (&raw const (*ptr).crc32).cast();
    let crc32_end: *const u8 = (&raw const (*ptr).reserved).cast();

    let mut crc = Crc32::new();
    for i in 0..(*ptr).header_size {
        let data = start.add(i as usize);
        if data >= crc32_start && data < crc32_end {
            crc.update(0x00);
        } else {
            crc.update(*data);
        }
    }
    assert_eq!(crc.finish(), (*ptr).crc32);
}
