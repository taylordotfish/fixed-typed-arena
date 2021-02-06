/*
 * Copyright (C) 2021 taylor.fish <contact@taylor.fish>
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
use alloc::alloc::Layout;
use core::marker::PhantomData;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use typenum::Unsigned;

// Invariants:
//
// * `self.0` points to memory allocated by the global allocator with the
//   layout `Self::layout()`.
//
// * The memory pointed to by `self.0` is not accessible except through a
//   single instance of this type. (In particular, there must not be two
//   instances that refer to the same memory.)
#[repr(transparent)]
pub struct ChunkMemory<T, Size: Unsigned>(
    NonNull<u8>,
    PhantomData<(NonNull<T>, *const Size)>,
);

impl<T, Size: Unsigned> ChunkMemory<T, Size> {
    pub fn new() -> Option<Self> {
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

    pub fn prev(&mut self) -> &mut MaybeUninit<Option<Chunk<T, Size>>> {
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
    /// large enough to hold at least `Size` (specifically,
    /// [`Size::USIZE`][USIZE]) aligned values of type `T`. Note that the
    /// memory could be uninitialized.
    ///
    /// [USIZE]: Unsigned::USIZE
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
        mem::size_of::<Option<Chunk<T, Size>>>()
    }

    fn prev_align() -> usize {
        mem::align_of::<Option<Chunk<T, Size>>>()
    }

    fn storage_size() -> usize {
        mem::size_of::<T>().checked_mul(Size::USIZE).expect("size overflow")
    }

    /// # Safety
    ///
    /// This function should be used only by the implementation of [`Drop`].
    /// It's available as a private method to reduce code duplication from the
    /// fact that we conditionally compile one of two [`Drop`] implementations
    /// depending on whether we can use `may_dangle`.
    unsafe fn drop(&mut self) {
        // SAFETY: `self.0` always points to memory allocated by the global
        // allocator with the layout `Self::layout()`.
        unsafe {
            alloc::alloc::dealloc(self.0.as_ptr(), Self::layout());
        }
    }
}

#[cfg(not(feature = "dropck_eyepatch"))]
impl<T, Size: Unsigned> Drop for ChunkMemory<T, Size> {
    fn drop(&mut self) {
        // SAFETY: `ChunkMemory::drop` is intended to be called by the
        // implementation of `Drop`. See that method's documentation.
        unsafe {
            ChunkMemory::drop(self);
        }
    }
}

#[cfg(feature = "dropck_eyepatch")]
unsafe impl<#[may_dangle] T, Size: Unsigned> Drop for ChunkMemory<T, Size> {
    fn drop(&mut self) {
        // SAFETY: `ChunkMemory::drop` is intended to be called by the
        // implementation of `Drop`. See that method's documentation.
        unsafe {
            ChunkMemory::drop(self);
        }
    }
}
