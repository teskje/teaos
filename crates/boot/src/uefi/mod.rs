//! A safe UEFI API wrapper.

pub mod sys;

mod bs_ref;

use alloc::vec::Vec;
use bs_ref::BsRef;
use core::ffi::c_void;
use core::{fmt, mem, ptr};

use crate::crc32::Crc32;
use crate::sync::Mutex;

static UEFI: Mutex<Option<Uefi>> = Mutex::new(None);

/// The number of references to boot services.
///
/// `None` if boot services are not available.
static BOOT_SERVICE_REFS: Mutex<Option<u64>> = Mutex::new(None);

struct Uefi {
    image_handle: sys::HANDLE,
    system_table: *mut sys::SYSTEM_TABLE,
}

impl Uefi {
    fn borrow<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Uefi) -> R,
    {
        let mut uefi = UEFI.lock();
        let uefi = uefi
            .as_mut()
            .unwrap_or_else(|| panic!("UEFI not initialized"));
        f(uefi)
    }

    fn console_out(&self) -> ConsoleOut {
        unsafe {
            let ptr = (*self.system_table).con_out;
            ConsoleOut::new(ptr)
        }
    }

    fn boot_services(&self) -> BootServices {
        unsafe {
            let ptr = (*self.system_table).boot_services;
            BootServices::new(ptr)
        }
    }

    fn config_table(&self) -> ConfigTable {
        unsafe {
            let ptr = (*self.system_table).configuration_table;
            let len = (*self.system_table).number_of_table_entries;
            ConfigTable::new(ptr, len)
        }
    }
}

/// # Safety
///
/// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
pub unsafe fn init(image_handle: sys::HANDLE, system_table: *mut sys::SYSTEM_TABLE) {
    validate_mut_ptr(system_table);
    validate_table_header(&raw const (*system_table).hdr, sys::SYSTEM_TABLE_SIGNATURE);

    *UEFI.lock() = Some(Uefi {
        image_handle,
        system_table,
    });
    *BOOT_SERVICE_REFS.lock() = Some(0);
}

pub fn exit_boot_services(map_key: usize) {
    Uefi::borrow(|uefi| unsafe {
        uefi.boot_services()
            .exit_boot_services(uefi.image_handle, map_key);
    });

    let refs_left = BOOT_SERVICE_REFS.lock().take().unwrap();
    if refs_left != 0 {
        panic!("{refs_left} boot service refs left after exit_boot_services");
    }
}

pub fn console_out() -> ConsoleOut {
    Uefi::borrow(|uefi| uefi.console_out())
}

pub fn boot_services() -> BootServices {
    Uefi::borrow(|uefi| uefi.boot_services())
}

pub fn config_table() -> ConfigTable {
    Uefi::borrow(|uefi| uefi.config_table())
}

pub struct ConsoleOut {
    ptr: BsRef<*mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL>,
}

impl ConsoleOut {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::SIMPLE_TEXT_OUTPUT_PROTOCOL`].
    unsafe fn new(ptr: *mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL) -> Self {
        validate_mut_ptr(ptr);

        Self {
            ptr: BsRef::new(ptr),
        }
    }
}

impl fmt::Write for ConsoleOut {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let ptr = *self.ptr;
        let output_string = unsafe { (*ptr).output_string };

        for c in s.encode_utf16() {
            let s = [c, 0x0000];
            let status = output_string(ptr, s.as_ptr());

            if status != sys::SUCCESS {
                return Err(fmt::Error);
            }
        }

        Ok(())
    }
}

pub struct BootServices {
    ptr: BsRef<*mut sys::BOOT_SERVICES>,
}

impl BootServices {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::BOOT_SERVICES`].
    unsafe fn new(ptr: *mut sys::BOOT_SERVICES) -> Self {
        validate_mut_ptr(ptr);
        validate_table_header(&raw const (*ptr).hdr, sys::BOOT_SERVICES_SIGNATURE);

        Self {
            ptr: BsRef::new(ptr),
        }
    }

    pub fn get_memory_map(&self, mut buffer: Vec<u8>) -> Result<MemoryMap, usize> {
        let get_memory_map = unsafe { (**self.ptr).get_memory_map };

        let mut buffer_size = buffer.len();
        let mut map_key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = get_memory_map(
            &mut buffer_size,
            buffer.as_mut_ptr().cast(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        assert_eq!(descriptor_version, sys::MEMORY_DESCRIPTOR_VERSION);

        if status == sys::BUFFER_TOO_SMALL {
            return Err(buffer_size);
        }

        assert_eq!(status, sys::SUCCESS);

        buffer.truncate(buffer_size);

        let memory_map = unsafe { MemoryMap::new(buffer, descriptor_size, map_key) };
        Ok(memory_map)
    }

    pub fn allocate_pool(&self, size: usize) -> *mut u8 {
        let allocate_pool = unsafe { (**self.ptr).allocate_pool };

        let mut buffer = ptr::null_mut();
        let status = allocate_pool(sys::LoaderData, size, &mut buffer);
        assert_eq!(status, sys::SUCCESS);

        buffer.cast()
    }

    pub fn free_pool(&self, ptr: *mut u8) {
        let free_pool = unsafe { (**self.ptr).free_pool };

        let status = free_pool(ptr.cast());
        assert_eq!(status, sys::SUCCESS);
    }

    /// # Safety
    ///
    /// Calling this method invalidates any references to the boot services and protocols. Callers
    /// must ensure that all such references have been dropped or are otherwise not used anymore.
    unsafe fn exit_boot_services(self, image_handle: sys::HANDLE, map_key: usize) {
        let exit_boot_services = unsafe { (**self.ptr).exit_boot_services };

        let status = exit_boot_services(image_handle, map_key);
        assert_eq!(status, sys::SUCCESS);
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
        validate_mut_ptr(ptr);

        Self { ptr, len }
    }

    pub fn iter(&self) -> impl Iterator<Item = (sys::GUID, *mut c_void)> + '_ {
        (0..self.len).into_iter().map(|i| unsafe {
            let ptr = self.ptr.add(i);
            ((*ptr).vendor_guid, (*ptr).vendor_table)
        })
    }
}

#[derive(Debug)]
pub struct MemoryMap {
    buffer: Vec<u8>,
    descriptor_size: usize,
    pub map_key: usize,
}

impl MemoryMap {
    /// # Safety
    ///
    /// `buffer` must be filled with [`sys::MEMORY_DESCRIPTOR`]s, each of which is padded up to to
    /// `descriptor_size`.
    unsafe fn new(buffer: Vec<u8>, descriptor_size: usize, map_key: usize) -> Self {
        assert!(descriptor_size > mem::size_of::<sys::MEMORY_DESCRIPTOR>());

        Self {
            buffer,
            descriptor_size,
            map_key,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &sys::MEMORY_DESCRIPTOR> {
        self.buffer.chunks(self.descriptor_size).map(|chunk| {
            let ptr: *const sys::MEMORY_DESCRIPTOR = chunk.as_ptr().cast();
            validate_ptr(ptr);

            unsafe { &*ptr }
        })
    }
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
