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

use super::Arena;
use core::mem::ManuallyDrop;
use typenum::{Unsigned, U16};

/// Like [`Arena`], but returns references of any lifetime, including
/// `'static`.
///
/// This lets the arena be used without being borrowed, but it comes with the
/// tradeoff that the arena leaks memory unless the unsafe [`drop`](Self::drop)
/// method is called.
pub struct ManuallyDropArena<T, ChunkSize: Unsigned = U16>(
    ManuallyDrop<Arena<T, ChunkSize>>,
);

impl<T, ChunkSize: Unsigned> Default for ManuallyDropArena<T, ChunkSize> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, ChunkSize: Unsigned> ManuallyDropArena<T, ChunkSize> {
    /// Creates a new [`ManuallyDropArena`].
    pub fn new() -> Self {
        Self(ManuallyDrop::new(Arena::new()))
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a reference to the allocated item. The reference can have any
    /// lifetime, including `'static`, as long as `T` outlives that lifetime.
    #[must_use]
    pub fn alloc<'a>(&mut self, value: T) -> &'a mut T {
        // SAFETY: `Arena` allocates all items on the heap, and we don't drop
        // the arena automatically (wrapped in `ManuallyDrop`), so it is safe
        // to extend the lifetime of the returned reference.
        unsafe { &mut *(self.0.alloc(value) as *mut _) }
    }

    /// Drops the contents of the arena. The arena will leak memory when
    /// dropped unless this method is called.
    ///
    /// # Safety
    ///
    /// You must ensure that no references to items (or parts of items) in the
    /// arena exist when calling this method, except possibly for references
    /// within the items themselves.
    ///
    /// However, if there are references to other items (or parts of items)
    /// within the items themselves, at least one of the following must be
    /// true:
    ///
    /// * `T` does not have a custom [`Drop`] impl.
    /// * `T`’s [`Drop`] impl does not directly or indirectly access any data
    ///   via the references to other items or parts of items. (This is
    ///   essentially the requirement imposed by [`#[may_dangle]`][dropck].)
    ///
    /// [dropck]: https://doc.rust-lang.org/nomicon/dropck.html
    pub unsafe fn drop(&mut self) {
        core::mem::take(&mut *self.0);
    }
}
