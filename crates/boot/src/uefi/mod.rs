//! A safe UEFI API wrapper.

pub mod boot_services;
pub mod protocol;
pub mod sys;

mod bs_ref;
mod string;

use alloc::vec;
use alloc::vec::Vec;
use boot_services::BootServices;
use core::ffi::c_void;
use core::mem;
use protocol::{ConsoleOut, FileSystem};

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
    unsafe {
        boot_services().exit_boot_services(image_handle(), map_key);
    }

    let refs_left = BOOT_SERVICE_REFS.lock().take().unwrap();
    if refs_left != 0 {
        panic!("{refs_left} boot service refs left after exit_boot_services");
    }
}

pub fn image_handle() -> sys::HANDLE {
    Uefi::borrow(|uefi| uefi.image_handle)
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

pub fn get_memory_map() -> MemoryMap {
    let bs = boot_services();

    // Get the memory map size.
    let mut buffer_size = bs.get_memory_map(vec![]).unwrap_err();

    // Allocate a sufficiently large buffer.
    //
    // "The actual size of the buffer allocated for the consequent call to `GetMemoryMap()`
    // should be bigger then the value returned in `MemoryMapSize`, since allocation of the new
    // buffer may potentially increase memory map size."
    buffer_size += 1024;
    let buffer: Vec<u8> = vec![0; buffer_size];

    // Get the memory map.
    bs.get_memory_map(buffer).expect("buffer large enough")
}

pub fn get_boot_fs() -> FileSystem {
    let bs = boot_services();

    let loaded_image = bs.get_loaded_image(image_handle());
    let boot_device = loaded_image.device_handle();
    bs.get_file_system(boot_device)
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
