use core::ffi::CStr;
use core::mem;

use crate::error::{Error, Result};

pub(crate) trait Read<'d> {
    fn read_n(&mut self, n: usize) -> Result<&'d [u8]>;
    fn read_until(&mut self, terminator: u8) -> &'d [u8];
    fn align_for<T>(&mut self);

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_n(N)?;
        let array = bytes.try_into().unwrap();
        Ok(array)
    }

    fn read_u32(&mut self) -> Result<u32> {
        self.read_array().map(u32::from_be_bytes)
    }

    fn read_u64(&mut self) -> Result<u64> {
        self.read_array().map(u64::from_be_bytes)
    }

    fn read_cstr(&mut self) -> Result<&'d CStr> {
        let bytes = self.read_until(0);
        CStr::from_bytes_until_nul(bytes).map_err(|_| Error::MissingNulTerminator)
    }
}

impl<'d> Read<'d> for &'d [u8] {
    fn read_n(&mut self, n: usize) -> Result<&'d [u8]> {
        if self.len() < n {
            return Err(Error::NotEnoughData);
        }

        let (bytes, rest) = self.split_at(n);

        *self = rest;
        Ok(bytes)
    }

    fn read_until(&mut self, terminator: u8) -> &'d [u8] {
        let n = match self.iter().position(|b| *b == terminator) {
            Some(idx) => idx + 1,
            None => self.len(),
        };
        self.read_n(n).unwrap()
    }

    fn align_for<T>(&mut self) {
        let align = mem::align_of::<T>();
        let addr = *self as *const _ as *const u8 as usize;
        let padding = (align - (addr % align)) % align;
        self.read_n(padding).ok();
    }
}
