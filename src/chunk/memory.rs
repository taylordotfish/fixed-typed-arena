/*
 * Copyright (C) 2021-2022 taylor.fish <contact@taylor.fish>
 *
 * This file is part of fixed-typed-arena.
 *
 * fixed-typed-arena is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * fixed-typed-arena is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with fixed-typed-arena. If not, see <https://www.gnu.org/licenses/>.
 */

use super::Chunk;
use alloc::alloc::{handle_alloc_error, Layout};
use alloc::boxed::Box;
use core::marker::PhantomData;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

// Invariants:
//
// * `self.0` points to memory allocated by the global allocator with the
//   layout `Self::layout()`.
//
// * The memory pointed to by `self.0` is not accessible except through a
//   single instance of this type. (In particular, there must not be two
//   instances that refer to the same memory.)
#[repr(transparent)]
pub struct ChunkMemory<T, const SIZE: usize>(
    NonNull<u8>,
    // Lets dropck know that `T` may be dropped.
    PhantomData<Box<T>>,
);

impl<T, const SIZE: usize> ChunkMemory<T, SIZE> {
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|| handle_alloc_error(Self::layout()))
    }

    pub fn try_new() -> Option<Self> {
        let layout = Self::layout();
        assert!(layout.size() > 0);
        Some(Self(
            // SAFETY: We ensured that `layout.size()` is non-zero above. This
            // should always be true, because the layout is at least as large
            // as `Self::prev_size()`.
            NonNull::new(unsafe { alloc::alloc::alloc(layout) })?,
            PhantomData,
        ))
    }

    pub fn prev(&mut self) -> &mut MaybeUninit<Option<Chunk<T, SIZE>>> {
        // SAFETY: `Self::prev_offset()` is never larger than the size of the
        // memory pointed to by `self.0`.
        let ptr = unsafe { self.0.as_ptr().add(Self::prev_offset()) };
        // SAFETY: `ptr` points to valid (but possibly uninitialized) memory
        // with proper alignment and enough space for an `Option<Chunk>`, so we
        // can cast to a `*mut MaybeUninit` and dereference.
        #[allow(clippy::cast_ptr_alignment)]
        unsafe {
            &mut *ptr.cast::<MaybeUninit<_>>()
        }
    }

    /// Returns a pointer to the start of the storage. It is guaranteed to be
    /// large enough to hold at least `SIZE` aligned values of type `T`. Note
    /// that the memory could be uninitialized.
    pub fn storage(&self) -> NonNull<T> {
        // SAFETY: `Self::storage_offset()` is never larger than the size
        // of the memory pointed to by `self.0`.
        let ptr = unsafe { self.0.as_ptr().add(Self::storage_offset()) };
        // SAFETY: `self.0` is non-null, so `ptr` must also be non-null.
        unsafe { NonNull::new_unchecked(ptr) }.cast()
    }

    fn layout() -> Layout {
        Layout::from_size_align(
            Self::prev_size().checked_add(Self::storage_size()).unwrap(),
            Self::align(),
        )
        .unwrap()
    }

    fn prev_offset() -> usize {
        if Self::prev_align() >= mem::align_of::<T>() {
            0
        } else {
            Self::storage_size()
        }
    }

    fn storage_offset() -> usize {
        if Self::prev_offset() == 0 {
            Self::prev_size()
        } else {
            0
        }
    }

    fn align() -> usize {
        mem::align_of::<T>().max(Self::prev_align())
    }

    fn prev_size() -> usize {
        mem::size_of::<Option<Chunk<T, SIZE>>>()
    }

    fn prev_align() -> usize {
        mem::align_of::<Option<Chunk<T, SIZE>>>()
    }

    fn storage_size() -> usize {
        mem::size_of::<T>().checked_mul(SIZE).expect("size overflow")
    }
}

macro_rules! drop_fn {
    () => {
        fn drop(&mut self) {
            // SAFETY: `self.0` always points to memory allocated by the global
            // allocator with the layout `Self::layout()`.
            unsafe {
                alloc::alloc::dealloc(self.0.as_ptr(), Self::layout());
            }
        }
    };
}

#[cfg(not(feature = "dropck_eyepatch"))]
impl<T, const SIZE: usize> Drop for ChunkMemory<T, SIZE> {
    drop_fn!();
}

// SAFETY: This `Drop` impl does not directly or indirectly access any data in
// any `T`, except for calling `T`'s destructor, and `Self` contains a
// `PhantomData<Box<T>>` so dropck knows that `T` may be dropped.
//
// See `ArenaInner`'s `Drop` impl for more information.
#[cfg(feature = "dropck_eyepatch")]
unsafe impl<#[may_dangle] T, const SIZE: usize> Drop for ChunkMemory<T, SIZE> {
    drop_fn!();
}

// SAFETY: `ChunkMemory` represents an owned region of memory (in particular,
// no two instances of `ChunkMemory` will point to the same region of memory),
// so it can be sent to another thread as long as `T` is `Send`.
unsafe impl<T: Send, const SIZE: usize> Send for ChunkMemory<T, SIZE> {}
