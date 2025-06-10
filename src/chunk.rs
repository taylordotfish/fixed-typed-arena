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

use alloc::alloc::Layout;
use alloc::boxed::Box;
use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::ptr::{NonNull, addr_of_mut};

struct Chunk<T, Array> {
    items: MaybeUninit<Array>,
    next: Option<ChunkRef<T, Array>>,
    phantom: PhantomData<T>,
}

// Invariant: `self.0` always points to a valid, initialized, properly aligned
// `Chunk`.
#[repr(transparent)]
pub struct ChunkRef<T, Array>(NonNull<Chunk<T, Array>>);

impl<T, Array> PartialEq for ChunkRef<T, Array> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, Array> Eq for ChunkRef<T, Array> {}

// This type is not `Copy` in order to make it easier to track clones, since
// `Self::dealloc` has safety requirements that require knowledge of all such
// clones that may exist.
impl<T, Array> Clone for ChunkRef<T, Array> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T, Array> ChunkRef<T, Array> {
    pub const CAPACITY: usize = mem::size_of::<Array>() / mem::size_of::<T>();
    pub const LAYOUT: Layout = Layout::new::<Chunk<T, Array>>();

    pub fn new(prev: Option<Self>) -> Option<Self> {
        assert!(mem::align_of::<Array>() >= mem::align_of::<T>());
        assert!(mem::size_of::<Array>() % mem::size_of::<T>() == 0);
        assert!(Self::LAYOUT.size() > 0);

        // SAFETY: We ensured `Self::LAYOUT` has non-zero size above.
        let ptr: NonNull<Chunk<T, Array>> =
            NonNull::new(unsafe { alloc::alloc::alloc(Self::LAYOUT) })?.cast();

        // SAFETY: `alloc::alloc::alloc` returns valid, properly aligned
        // memory.
        unsafe {
            addr_of_mut!((*ptr.as_ptr()).next).write(None);
        }

        let chunk = Self(ptr);
        if let Some(mut prev) = prev {
            prev.set_next(Some(chunk.clone()));
        }
        Some(chunk)
    }

    pub fn as_ptr(&self) -> NonNull<()> {
        self.0.cast()
    }

    /// # Safety
    ///
    /// * `ptr` must have come from a previous call to [`Self::as_ptr`].
    /// * The corresponding chunk must not have been deallocated.
    pub unsafe fn from_ptr(ptr: NonNull<()>) -> Self {
        Self(ptr.cast())
    }

    pub fn next(&self) -> Option<Self> {
        // SAFETY: `self.0` is always initialized and properly aligned.
        unsafe { &(*self.0.as_ptr()).next }.clone()
    }

    fn set_next(&mut self, next: Option<Self>) {
        // SAFETY: `self.0` is always initialized and properly aligned.
        unsafe {
            (*self.0.as_ptr()).next = next;
        }
    }

    /// Frees the memory in this chunk.
    ///
    /// # Safety
    ///
    /// After calling this method, all other [`ChunkRef`]s that point to the
    /// same memory as this [`ChunkRef`] (i.e., are clones of `self`) must
    /// never be accessed, except for being dropped. This is trivially true if
    /// no such clones exist.
    pub unsafe fn dealloc(self) {
        // SAFETY: `self.0` was allocated by `alloc::alloc::alloc` and can thus
        // be turned into a `Box`.
        drop(unsafe { Box::from_raw(self.0.as_ptr()) });
    }

    /// Returns a pointer to the item at index `i`. If `i` is less than
    /// [`Self::CAPACITY`], the pointer is guaranteed to be valid and properly
    /// aligned, but not necessarily initialized.
    ///
    /// # Safety
    ///
    /// `i` must be less than or equal to [`Self::CAPACITY`].
    pub unsafe fn get(&self, i: usize) -> NonNull<T> {
        // SAFETY: Index is checked by caller. Casting to a `NonNull<T>` is
        // fine, since `T` and `MaybeUninit<T>` have the same layout.
        unsafe { self.get_uninit(i).cast() }
    }

    /// Returns a pointer to the item at index `i`. If `i` is less than
    /// [`Self::CAPACITY`], the pointer is guaranteed to be valid and
    /// properly aligned.
    ///
    /// # Safety
    ///
    /// `i` must be less than or equal to [`Self::CAPACITY`].
    unsafe fn get_uninit(&self, i: usize) -> NonNull<MaybeUninit<T>> {
        debug_assert!(i <= Self::CAPACITY);

        // SAFETY: `self.0` is always valid and properly aligned. We use
        // `addr_of_mut` here to avoid creating a reference to the entire
        // array, since parts of the array may already be borrowed.
        let ptr = unsafe { addr_of_mut!((*self.0.as_ptr()).items) };

        // SAFETY: Caller guarantees `i` is in bounds, and `addr_of_mut` always
        // returns valid pointers when used correctly.
        unsafe { NonNull::new_unchecked(ptr.cast::<MaybeUninit<T>>().add(i)) }
    }

    /// # Safety
    ///
    /// * The item at index `i` must be initialized. Note that this method will
    ///   make it uninitialized.
    /// * It must be safe to drop the item at index `i`.
    pub unsafe fn drop_item(&mut self, i: usize) {
        // SAFETY: Caller guarantees that item `i` is initialized (which
        // requires `i` to be in bounds) and safe to drop.
        unsafe {
            self.get(i).as_ptr().drop_in_place();
        }
    }

    /// # Safety
    ///
    /// * All items in the chunk must be initialized. Note that this method
    ///   will make them uninitialized.
    /// * It must be safe to drop all items.
    pub unsafe fn drop_all(&mut self) {
        for i in 0..Self::CAPACITY {
            // SAFETY: Caller guarantees that all items are initialized and
            // safe to drop.
            unsafe {
                self.drop_item(i);
            }
        }
    }
}
