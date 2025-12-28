use alloc::vec;
use alloc::vec::Vec;
use core::{ptr, str};

use crate::exception::ExceptionStack;
use crate::log;
use crate::memory::virt::KERNEL_START;

pub(super) fn print(stack: &ExceptionStack) {
    let ptr = stack.x0 as *const u8;
    let len = stack.x1 as usize;

    let bytes = copy_from_user(ptr, len);
    let s = str::from_utf8(&bytes).unwrap();

    log::log_args(format_args!("{s}"), "user");
}

/// Copy user memory into kernel space.
fn copy_from_user(ptr: *const u8, len: usize) -> Vec<u8> {
    let end = (ptr as u64).checked_add(len as u64).unwrap();
    assert!(end < KERNEL_START.into());

    let mut buf = vec![0; len];
    unsafe { ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), len) };

    buf
}
