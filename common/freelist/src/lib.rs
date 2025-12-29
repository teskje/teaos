//! Implementation of a freelist, usable for implementing simple heap allocators.

#![no_std]

use core::mem;
use core::ptr::NonNull;

pub const ALIGN: usize = 16;

/// A linked list of free blocks of memory.
///
/// Invariants:
///  * Block boundaries are aligned to 16 bytes.
///  * Blocks are sorted in address order.
///  * Blocks are maximally coalesced.
pub struct FreeList {
    head: Option<NonNull<FreeBlock>>,
}

impl FreeList {
    /// Create a new, empty freelist.
    pub const fn new() -> Self {
        Self { head: None }
    }

    /// Carve a block out of the freelist.
    ///
    /// This searches through the freelist until it finds a block that is at least as large as the
    /// requested size (first fit), then splits that block, if necessary, and returns it.
    ///
    /// # Panics
    ///
    /// Panics if `size` is not a multiple of [`ALIGN`].
    pub fn carve(&mut self, size: usize) -> Option<NonNull<u8>> {
        assert!(size.is_multiple_of(ALIGN), "invalid size: {size}");

        let mut head = &mut self.head;

        while let Some(mut block_ptr) = *head {
            let block = unsafe { block_ptr.as_mut() };

            if block.size == size {
                *head = block.next;
                return Some(block_ptr.cast());
            }

            if block.size > size {
                let rest = block.size - size;
                debug_assert!(rest >= mem::size_of::<FreeBlock>());

                unsafe {
                    let new_block_ptr = block_ptr.byte_add(size);
                    new_block_ptr.write(FreeBlock {
                        size: rest,
                        next: block.next,
                    });
                    *head = Some(new_block_ptr);
                }

                return Some(block_ptr.cast());
            }

            head = &mut block.next;
        }

        None
    }

    /// Insert a free block into the freelist.
    ///
    /// # Safety
    ///
    /// This transfers ownership of the given memory block to the freelist. Consequently, the
    /// memory block must not have any other users.
    ///
    /// # Panics
    ///
    /// Panics if `ptr` is not aligned to [`ALIGN`].
    /// Panics if `size` is not a multiple of [`ALIGN`].
    pub unsafe fn insert(&mut self, ptr: NonNull<u8>, size: usize) {
        assert_eq!(ptr.align_offset(ALIGN), 0);
        assert!(size.is_multiple_of(ALIGN), "invalid size: {size}");

        if size == 0 {
            return;
        }

        debug_assert!(size >= mem::size_of::<FreeBlock>());

        let mut new_block_ptr = ptr.cast();

        // Find the insertion point, according to address order. Also keep track of the previous
        // block, we might need it for coalescing.
        let mut prev = None;
        let mut this = &mut self.head;
        while let Some(mut block_ptr) = *this {
            if block_ptr > new_block_ptr {
                break;
            }

            let block = unsafe { block_ptr.as_mut() };
            this = &mut block.next;
            prev = Some(block_ptr);
        }

        // Insert the new block.
        unsafe { new_block_ptr.write(FreeBlock { size, next: *this }) };
        *this = Some(new_block_ptr);

        // Coalesce with neighbors, if possible.
        unsafe { new_block_ptr.as_mut().coalesce() };
        if let Some(mut prev_ptr) = prev {
            unsafe { prev_ptr.as_mut().coalesce() };
        }
    }
}

/// Header for a block in a [`FreeList`].
struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>,
}

impl FreeBlock {
    /// Coalesce this block with the next one, if possible.
    fn coalesce(&mut self) {
        let Some(next_start) = self.next else {
            return;
        };

        let this_start = NonNull::from_mut(self);
        let this_end = unsafe { this_start.byte_add(self.size) };

        if this_end == next_start {
            let next = unsafe { next_start.as_ref() };
            self.size += next.size;
            self.next = next.next;
        }
    }
}

pub fn round_up_align(x: usize) -> usize {
    debug_assert!(ALIGN.is_power_of_two());
    let a = ALIGN - 1;
    (x + a) & !a
}
