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

use core::mem::MaybeUninit;
use core::ptr::NonNull;
use typenum::Unsigned;

mod memory;
use memory::ChunkMemory;

#[repr(transparent)]
pub struct Chunk<T, Size: Unsigned>(ChunkMemory<T, Size>);

impl<T, Size: Unsigned> Chunk<T, Size> {
    pub fn new(prev: Option<Self>) -> Option<Self> {
        let mut memory = ChunkMemory::new()?;
        *memory.prev() = MaybeUninit::new(prev);
        Some(Self(memory))
    }

    pub fn into_prev(self) -> Option<Self> {
        let mut memory = self.0;
        // SAFETY: `memory.prev()` must be initialized due to this type's
        // invariants. (In particular, `Self::new` initializes it.)
        unsafe { memory.prev().as_ptr().read() }
    }

    /// Returns a pointer to the item at index `i`. The memory is valid and
    /// properly aligned, but may be uninitialized.
    ///
    /// # Safety
    ///
    /// `i` must be less than `Size` (specifically, [`Size::USIZE`][USIZE]).
    ///
    /// [USIZE]: Unsigned::USIZE
    pub unsafe fn get(&self, i: usize) -> NonNull<T> {
        let storage: NonNull<T> = self.0.storage();
        // SAFETY: Caller guarantees `i` is in bounds.
        let ptr = unsafe { storage.as_ptr().add(i) };
        // SAFETY: `storage` is non-null, so `ptr` must also be non-null.
        unsafe { NonNull::new_unchecked(ptr) }
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
        for i in (0..Size::USIZE).rev() {
            // SAFETY: Caller guarantees that all items are initialized and
            // safe to drop.
            unsafe {
                self.drop_item(i);
            }
        }
    }
}
