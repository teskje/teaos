//! The [`BsRef`] type for references to boot services.
//!
//! Boot service references are counted, so we can check that none exit anymore when
//! [`super::exit_boot_services`] was called.

use core::ops::Deref;

#[repr(transparent)]
pub(super) struct BsRef<T>(T);

impl<T> BsRef<T> {
    pub fn new(inner: T) -> Self {
        inc_boot_service_refs();
        Self(inner)
    }
}

impl<T> Drop for BsRef<T> {
    fn drop(&mut self) {
        dec_boot_service_refs();
    }
}

impl<T> Deref for BsRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

fn inc_boot_service_refs() {
    match &mut *super::BOOT_SERVICE_REFS.lock() {
        Some(count) => *count += 1,
        None => panic!("boot services not available"),
    }
}

fn dec_boot_service_refs() {
    match &mut *super::BOOT_SERVICE_REFS.lock() {
        Some(count) => *count -= 1,
        None => panic!("boot services not available"),
    }
}
