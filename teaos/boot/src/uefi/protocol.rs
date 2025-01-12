use core::{fmt, ptr};

use kstd::io::{self, Read, Seek};

use super::bs_ref::BsRef;
use super::string::String;
use super::{sys, validate_mut_ptr};

pub struct LoadedImage {
    ptr: BsRef<*mut sys::LOADED_IMAGE_PROTOCOL>,
}

impl LoadedImage {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::LOADED_IMAGE_PROTOCOL`].
    pub unsafe fn new(ptr: *mut sys::LOADED_IMAGE_PROTOCOL) -> Self {
        validate_mut_ptr(ptr);
        assert_eq!((*ptr).revision, sys::LOADED_IMAGE_PROTOCOL_REVISION);

        Self {
            ptr: BsRef::new(ptr),
        }
    }

    pub fn device_handle(&self) -> sys::HANDLE {
        unsafe { (**self.ptr).device_handle }
    }
}

pub struct ConsoleOut {
    ptr: BsRef<*mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL>,
}

impl ConsoleOut {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::SIMPLE_TEXT_OUTPUT_PROTOCOL`].
    pub unsafe fn new(ptr: *mut sys::SIMPLE_TEXT_OUTPUT_PROTOCOL) -> Self {
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

        let s = String::from(s);
        let status = output_string(ptr, s.as_ptr());
        if status != sys::SUCCESS {
            return Err(fmt::Error);
        }

        Ok(())
    }
}

pub struct FileSystem {
    ptr: BsRef<*mut sys::SIMPLE_FILE_SYSTEM_PROTOCOL>,
}

impl FileSystem {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::SIMPLE_FILE_SYSTEM_PROTOCOL`].
    pub unsafe fn new(ptr: *mut sys::SIMPLE_FILE_SYSTEM_PROTOCOL) -> Self {
        validate_mut_ptr(ptr);
        assert_eq!((*ptr).revision, sys::SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION);

        Self {
            ptr: BsRef::new(ptr),
        }
    }

    pub fn open_volume(&self) -> File {
        let open_volume = unsafe { (**self.ptr).open_volume };

        let mut root = ptr::null_mut();
        let status = open_volume(*self.ptr, &mut root);
        assert_eq!(status, sys::SUCCESS);

        unsafe { File::new(root) }
    }
}

pub struct File {
    ptr: BsRef<*mut sys::FILE_PROTOCOL>,
}

impl File {
    /// # Safety
    ///
    /// `ptr` must be a valid pointer to a [`sys::FILE_PROTOCOL`].
    pub unsafe fn new(ptr: *mut sys::FILE_PROTOCOL) -> Self {
        validate_mut_ptr(ptr);
        assert!((*ptr).revision >= sys::FILE_PROTOCOL_REVISION);

        Self {
            ptr: BsRef::new(ptr),
        }
    }

    pub fn open(&self, file_name: &str) -> File {
        let open = unsafe { (**self.ptr).open };

        let file_name = String::from(file_name);
        let mut new_handle = ptr::null_mut();
        let status = open(
            *self.ptr,
            &mut new_handle,
            file_name.as_ptr(),
            sys::FILE_MODE_READ,
            0,
        );
        assert_eq!(status, sys::SUCCESS);

        unsafe { Self::new(new_handle) }
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let read = unsafe { (**self.ptr).read };

        let mut buf_size = buf.len();
        let status = read(*self.ptr, &mut buf_size, buf.as_mut_ptr().cast());
        assert_eq!(status, sys::SUCCESS);

        Ok(buf_size)
    }
}

impl Seek for File {
    fn seek(&mut self, pos: u64) -> Result<(), io::Error> {
        let set_position = unsafe { (**self.ptr).set_position };

        let status = set_position(*self.ptr, pos);
        assert_eq!(status, sys::SUCCESS);

        Ok(())
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let close = unsafe { (**self.ptr).close };

        let status = close(*self.ptr);
        assert_eq!(status, sys::SUCCESS);
    }
}
