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

use super::inner::ArenaInner;
use core::cell::UnsafeCell;
use typenum::{Unsigned, U16};

/// An arena that allocates items of type `T` in non-amortized O(1) (constant)
/// time.
///
/// The arena allocates fixed-size chunks of memory, each able to hold up to
/// `ChunkSize` items. `ChunkSize` is an unsigned
/// [type-level integer](typenum).
///
/// All items are allocated on the heap.
///
/// # Panics
///
/// The arena may panic when created or used if
/// [`mem::size_of::<T>()`](core::mem::size_of) times
/// [`ChunkSize::USIZE`](Unsigned::USIZE) is greater than [`usize::MAX`].
pub struct Arena<T, ChunkSize: Unsigned = U16>(
    UnsafeCell<ArenaInner<T, ChunkSize>>,
);

impl<T, ChunkSize: Unsigned> Default for Arena<T, ChunkSize> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, ChunkSize: Unsigned> Arena<T, ChunkSize> {
    /// Creates a new [`Arena`].
    pub fn new() -> Self {
        Self(UnsafeCell::new(ArenaInner::new()))
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a reference to the allocated item.
    #[must_use]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc(&self, value: T) -> &mut T {
        // SAFETY: `ArenaInner::alloc` does not run any code that could
        // possibly call any methods of `Self`, which ensures that we
        // do not borrow the data in the `UnsafeCell` multiple times
        // concurrently.
        //
        // Additionally, the memory pointed to by the mutable reference we
        // return is guaranteed by the implementation of `ArenaInner` not to
        // change (except through the reference itself) or be reused until the
        // `ArenaInner` is dropped, which will not happen until `*self` is
        // dropped (which itself cannot happen while references returned by
        // `Self::alloc` are still alive).
        unsafe { &mut *self.0.get() }.alloc(value)
    }
}
