//! Thread synchronization primitives.
//!
//! For now, both boot loader and kernel are single-threaded, so the "synchronization" merely
//! consists of asserting that there is no concurrenct access to the protected data.

#![no_std]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// A simple lock.
pub struct Lock {
    locked: AtomicBool,
}

impl Lock {
    pub const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self) {
        let was_locked = self.locked.swap(true, Ordering::SeqCst);
        assert!(!was_locked);
    }

    pub fn unlock(&self) {
        self.locked.swap(false, Ordering::SeqCst);
    }
}

/// A wrapper that ensures exclusive access to the wrapped data.
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    lock: Lock,
}

unsafe impl<T> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            lock: Lock::new(),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.lock.lock();
        MutexGuard { lock: self }
    }
}

pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.unlock();
    }
}
