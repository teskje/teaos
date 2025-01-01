use core::ffi::c_void;
use core::ptr;

use super::bs_ref::BsRef;
use super::protocol::{FileSystem, LoadedImage};
use super::{sys, validate_mut_ptr, validate_table_header, MemoryMap};

use alloc::vec::Vec;

pub struct BootServices {
    ptr: BsRef<*mut sys::BOOT_SERVICES>,
}

impl BootServices {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::BOOT_SERVICES`].
    pub unsafe fn new(ptr: *mut sys::BOOT_SERVICES) -> Self {
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

    pub fn handle_protocol(&self, handle: sys::HANDLE, protocol: &sys::GUID) -> *mut c_void {
        // SAFETY: `self.ptr` is a valid pointer to a `sys::BOOT_SERVICES`.
        let handle_protocol = unsafe { (**self.ptr).handle_protocol };

        let mut interface = ptr::null_mut();
        let status = handle_protocol(handle, protocol, &mut interface);
        assert_eq!(status, sys::SUCCESS);

        interface
    }

    pub fn get_loaded_image(&self, handle: sys::HANDLE) -> LoadedImage {
        let ptr = self.handle_protocol(handle, &sys::LOADED_IMAGE_PROTOCOL_GUID);
        unsafe { LoadedImage::new(ptr.cast()) }
    }

    pub fn get_file_system(&self, handle: sys::HANDLE) -> FileSystem {
        let ptr = self.handle_protocol(handle, &sys::SIMPLE_FILE_SYSTEM_PROTOCOL_GUID);
        unsafe { FileSystem::new(ptr.cast()) }
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
    pub unsafe fn exit_boot_services(self, image_handle: sys::HANDLE, map_key: usize) {
        let exit_boot_services = unsafe { (**self.ptr).exit_boot_services };

        let status = exit_boot_services(image_handle, map_key);
        assert_eq!(status, sys::SUCCESS);
    }
}
