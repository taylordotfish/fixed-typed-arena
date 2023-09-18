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

//! A typed arena that allocates items in non-amortized constant time.

use super::iter::{IntoIter, Iter, IterMut, Position};
use super::manually_drop::ManuallyDropArena;
use super::ArenaOptions;
use core::cell::UnsafeCell;
use core::mem::ManuallyDrop;
use integral_constant::Bool;

/// An arena that allocates items of type `T` in non-amortized O(1) (constant)
/// time.
///
/// The arena allocates fixed-size chunks of memory, each able to hold up to
/// [`Options::ChunkSize`] items. All items are allocated on the heap.
///
/// # Panics
///
/// The arena may panic when created or used if [`mem::size_of::<T>()`][size]
/// times [`Options::ChunkSize`] is greater than [`usize::MAX`].
///
/// [`Options::ChunkSize`]: ArenaOptions::ChunkSize
/// [size]: core::mem::size_of
pub struct Arena<T, Options: ArenaOptions<T> = super::Options>(
    ManuallyDrop<UnsafeCell<ManuallyDropArena<T, Options>>>,
);

impl<T, Options: ArenaOptions<T>> Default for Arena<T, Options> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Options: ArenaOptions<T>> Arena<T, Options> {
    /// Creates a new [`Arena`].
    pub fn new() -> Self {
        Self(Default::default())
    }

    fn inner(&self) -> &ManuallyDropArena<T, Options> {
        // SAFETY: No `&self` methods of `ManuallyDropArena` can possibly call
        // any methods of `Self`, which ensures we do not concurrently mutably
        // borrow the data in the `UnsafeCell`.
        unsafe { &*self.0.get() }
    }

    /// Returns the total number of items that have been allocated.
    pub fn len(&self) -> usize {
        self.inner().len()
    }

    /// Checks whether the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a reference to the allocated item.
    ///
    /// This method calls [`handle_alloc_error`] if memory allocation fails;
    /// for a version that returns [`None`] instead, see [`Self::try_alloc`].
    ///
    /// [`handle_alloc_error`]: alloc::alloc::handle_alloc_error
    #[allow(clippy::mut_from_ref)]
    pub fn alloc(&self, value: T) -> &mut T
    where
        Options: ArenaOptions<T, Mutable = Bool<true>>,
    {
        // SAFETY: `ManuallyDropArena::alloc` does not run any code that could
        // possibly call any methods of `Self`, which ensures that we do not
        // borrow the data in the `UnsafeCell` multiple times concurrently.
        //
        // Additionally, the memory pointed to by the mutable reference we
        // return is guaranteed by the implementation of `ManuallyDropArena`
        // not to change (except through the reference itself) or be reused
        // until either the arena is dropped, or until `Self::iter_mut` is
        // called, both of which require mutable access to the arena, which
        // require no live item references to exist.
        unsafe { &mut *self.0.get() }.alloc(value)
    }

    /// Like [`Self::alloc`], but returns [`None`] if memory allocation fails.
    #[allow(clippy::mut_from_ref)]
    pub fn try_alloc(&self, value: T) -> Option<&mut T>
    where
        Options: ArenaOptions<T, Mutable = Bool<true>>,
    {
        // SAFETY: See `Self::alloc`.
        unsafe { &mut *self.0.get() }.try_alloc(value)
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a shared/immutable reference to the allocated item.
    ///
    /// This method calls [`handle_alloc_error`] if memory allocation fails;
    /// for a version that returns [`None`] instead, see [`Self::try_alloc`].
    ///
    /// [`handle_alloc_error`]: alloc::alloc::handle_alloc_error
    pub fn alloc_shared(&self, value: T) -> &T {
        // SAFETY: See `Self::alloc`.
        unsafe { &mut *self.0.get() }.alloc_shared(value)
    }

    /// Like [`Self::alloc_shared`], but returns [`None`] if memory allocation
    /// fails.
    pub fn try_alloc_shared(&self, value: T) -> Option<&T> {
        // SAFETY: See `Self::alloc_shared`.
        unsafe { &mut *self.0.get() }.try_alloc_shared(value)
    }

    /// Returns an iterator over the items in this arena.
    pub fn iter(&self) -> Iter<'_, T, Options>
    where
        Options: ArenaOptions<T, Mutable = Bool<false>>,
    {
        self.inner().iter()
    }

    /// Returns an iterator over the items in this arena.
    ///
    /// # Safety
    ///
    /// There must be no mutable references (or references derived from mutable
    /// references) to items (or parts of items) in this arena or instances of
    /// [`IterMut`] for this arena.
    pub unsafe fn iter_unchecked(&self) -> Iter<'_, T, Options> {
        // SAFETY: Checked by caller.
        unsafe { self.inner().iter_unchecked() }
    }

    /// Returns a mutable iterator over the items in this arena.
    pub fn iter_mut(&mut self) -> IterMut<'_, T, Options> {
        // SAFETY: This type's design guarantees no references to items exist.
        unsafe { self.0.get_mut().iter_mut_unchecked() }
    }
}

impl<T, Options> Arena<T, Options>
where
    Options: ArenaOptions<T, SupportsPositions = Bool<true>>,
{
    /// Returns an iterator starting at the specified position.
    ///
    /// # Panics
    ///
    /// May panic if `position` does not refer to a position in this arena.
    pub fn iter_at(&self, position: &Position) -> Iter<'_, T, Options>
    where
        Options: ArenaOptions<T, Mutable = Bool<false>>,
    {
        self.inner().iter_at(position)
    }

    /// Returns an iterator starting at the specified position.
    ///
    /// # Panics
    ///
    /// May panic if `position` does not refer to a position in this arena.
    ///
    /// # Safety
    ///
    /// Same requirements as [`Self::iter_unchecked`].
    pub unsafe fn iter_at_unchecked(
        &self,
        position: &Position,
    ) -> Iter<'_, T, Options> {
        // SAFETY: Checked by caller.
        unsafe { self.inner().iter_at_unchecked(position) }
    }

    /// Returns a mutable iterator starting at the specified position.
    ///
    /// # Panics
    ///
    /// May panic if `position` does not refer to a position in this arena.
    pub fn iter_mut_at(
        &mut self,
        position: &Position,
    ) -> IterMut<'_, T, Options> {
        // SAFETY: This type's design guarantees no references to items exist.
        unsafe { self.0.get_mut().iter_mut_at_unchecked(position) }
    }
}

// SAFETY: `Arena` owns its items and provides access using standard borrow
// rules, so it can be `Send` as long as `T` is `Send`.
unsafe impl<T, Options> Send for Arena<T, Options>
where
    T: Send,
    Options: ArenaOptions<T>,
{
}

// SAFETY: This `Drop` impl does not directly or indirectly access any data in
// any `T` or `Array`, except for calling their destructors (see [1]), and
// `Self` (via `ManuallyDropArena`) contains a `PhantomData<Box<T>>` so dropck
// knows that `T` may be dropped (see [2]).
//
// [1]: https://doc.rust-lang.org/nomicon/dropck.html
// [2]: https://forge.rust-lang.org/libs/maintaining-std.html
//      #is-there-a-manual-drop-implementation
#[cfg_attr(feature = "dropck_eyepatch", add_syntax::prepend(unsafe))]
impl<
    #[cfg_attr(feature = "dropck_eyepatch", may_dangle)] T,
    #[cfg_attr(feature = "dropck_eyepatch", may_dangle)] Options,
> Drop for Arena<T, Options>
where
    Options: ArenaOptions<T>,
{
    fn drop(&mut self) {
        // SAFETY: `Arena` doesn't hand out references or iterators
        // that live longer than itself.
        unsafe {
            self.0.get_mut().drop();
        }
    }
}

impl<'a, T, Options> IntoIterator for &'a Arena<T, Options>
where
    Options: ArenaOptions<T, Mutable = Bool<false>>,
{
    type IntoIter = Iter<'a, T, Options>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, Options> IntoIterator for &'a mut Arena<T, Options>
where
    Options: ArenaOptions<T>,
{
    type IntoIter = IterMut<'a, T, Options>;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, Options: ArenaOptions<T>> IntoIterator for Arena<T, Options> {
    type IntoIter = IntoIter<T, Options>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let mut this = ManuallyDrop::new(self);

        // SAFETY: This `ManuallyDrop` won't be used again because we moved
        // `self` into a `ManuallyDrop` (so its destructor won't run).
        let inner = unsafe { ManuallyDrop::take(&mut this.0) };

        // SAFETY: This type's design guarantees no references to items exist.
        unsafe { inner.into_inner().into_iter_unchecked() }
    }
}
