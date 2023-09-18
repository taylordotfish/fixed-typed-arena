/*
 * Copyright (C) 2022 taylor.fish <contact@taylor.fish>
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

use super::ManuallyDropArena;
use super::{ArenaChunk, ArenaRc};
use crate::chunk::ChunkRef;
use crate::ArenaOptions;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ptr::NonNull;
use integral_constant::Bool;

/// A position in an arena.
///
/// See [`IterMut::as_position`] and [`Arena::iter_mut_at`] (and the
/// corresponding methods for other iterator and arena types).
///
/// [`Arena::iter_mut_at`]: crate::arena::Arena::iter_mut_at
#[derive(Clone)]
pub struct Position {
    pub(super) chunk: Option<NonNull<()>>,
    pub(super) index: usize,
    pub(super) rc: Option<Arc<()>>,
}

// SAFETY: The caller must have access to the arena from which this position
// was derived in order to access the contained memory (the only non-`Send`
// field), so the position itself can be made `Send`.
unsafe impl Send for Position {}

// SAFETY: The caller must have access to the arena from which this position
// was derived in order to access the contained memory (the only non-`Sync`
// field), so the position itself can be made `Sync`.
unsafe impl Sync for Position {}

// Invariants:
//
// * All items in the list of chunks pointed to by `chunk` are initialized
//   until `end` is reached. `end` marks the *exclusive* end of the range of
//   initialized items.
// * If `DROP` is true, `chunk` is the only `ChunkRef` that refers to any chunk
//   in the corresponding arena.
// * `index` is always less than or equal to the chunk capacity.
pub(super) struct IterPtr<
    T,
    Options: ArenaOptions<T>,
    const DROP: bool = false,
> {
    pub chunk: Option<ArenaChunk<T, Options>>,
    pub index: usize,
    pub end: *const T,
    pub rc: Option<ArenaRc<T, Options>>,
    pub phantom: PhantomData<Box<T>>,
}

impl<T, Options, const DROP: bool> Clone for IterPtr<T, Options, DROP>
where
    Options: ArenaOptions<T>,
{
    fn clone(&self) -> Self {
        Self {
            chunk: self.chunk.clone(),
            index: self.index,
            end: self.end,
            rc: self.rc.clone(),
            phantom: self.phantom,
        }
    }
}

impl<T, Options, const DROP: bool> Iterator for IterPtr<T, Options, DROP>
where
    Options: ArenaOptions<T>,
{
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunk.clone()?;
        // SAFETY: `self.index` is always less than or equal to the chunk
        // capacity.
        let mut item = unsafe { chunk.get(self.index) };
        let end = self.end == item.as_ptr();

        if end || self.index >= ArenaChunk::<T, Options>::CAPACITY {
            let next = (!end).then(|| chunk.next()).flatten();
            if DROP || next.is_some() {
                self.index = 0;
                self.chunk = next.clone();
            }

            if DROP {
                // SAFETY: This type's invariants guarantee no other
                // `ChunkRef`s referring to chunks in this arena exist.
                unsafe {
                    chunk.dealloc();
                }
            }

            // SAFETY: `self.index` is always less than or equal to the chunk
            // capacity.
            item = unsafe { next?.get(self.index) };
        }

        self.index += 1;
        Some(item)
    }
}

#[rustfmt::skip]
impl<T, Options, const DROP: bool> FusedIterator for IterPtr<T, Options, DROP>
where
    Options: ArenaOptions<T>,
{
}

impl<T, Options, const DROP: bool> IterPtr<T, Options, DROP>
where
    Options: ArenaOptions<T, SupportsPositions = Bool<true>>,
{
    /// Note: The returned [`Position`] is tied to to the arena over which this
    /// [`IterPtr`] is iterating and can be turned back into an iterator with
    /// [`ManuallyDropArena::iter_ptr_at`].
    pub fn as_position(&self) -> Position {
        Position {
            chunk: self.chunk.as_ref().map(ChunkRef::as_ptr),
            index: self.index,
            rc: self.rc.clone(),
        }
    }
}

// SAFETY: This `Drop` impl does not directly or indirectly access any data in
// any `T` or `Options` (or associated types in `Options`) except for calling
// their destructors (see [1]), and `Self` contains a `PhantomData<Box<T>>` so
// dropck knows that `T` may be dropped (see [2]).
//
// [1]: https://doc.rust-lang.org/nomicon/dropck.html
// [2]: https://forge.rust-lang.org/libs/maintaining-std.html
//      #is-there-a-manual-drop-implementation
#[cfg_attr(feature = "dropck_eyepatch", add_syntax::prepend(unsafe))]
impl<
    #[cfg_attr(feature = "dropck_eyepatch", may_dangle)] T,
    #[cfg_attr(feature = "dropck_eyepatch", may_dangle)] Options,
    const DROP: bool,
> Drop for IterPtr<T, Options, DROP>
where
    Options: ArenaOptions<T>,
{
    fn drop(&mut self) {
        if !DROP {
            return;
        }
        for item in self {
            // SAFETY: This type yields initialized, properly aligned
            // pointers.
            unsafe {
                item.as_ptr().drop_in_place();
            }
        }
    }
}

/// An iterator over the items in an arena.
pub struct Iter<'a, T, Options: ArenaOptions<T>> {
    pub(super) inner: IterPtr<T, Options>,
    pub(super) phantom: PhantomData<&'a T>,
}

impl<T, Options> Iter<'_, T, Options>
where
    Options: ArenaOptions<T, SupportsPositions = Bool<true>>,
{
    /// Converts this iterator into a [`Position`]. The position can later be
    /// turned back into an iterator with [`Arena::iter_at`] (and similar
    /// methods).
    ///
    /// [`Arena::iter_at`]: crate::arena::Arena::iter_at
    pub fn as_position(&self) -> Position {
        self.inner.as_position()
    }
}

impl<'a, T, Options: ArenaOptions<T>> Iterator for Iter<'a, T, Options> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `IterPtr` always returns initialized, properly aligned
        // pointers.
        Some(unsafe { self.inner.next()?.as_ref() })
    }
}

impl<T, Options: ArenaOptions<T>> FusedIterator for Iter<'_, T, Options> {}

impl<T, Options: ArenaOptions<T>> Clone for Iter<'_, T, Options> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: self.phantom,
        }
    }
}

// SAFETY: This type yields immutable references to items in the arena, so it
// can be `Send` as long as `T` is `Sync` (which means `&T` is `Send`).
unsafe impl<T, Options> Send for Iter<'_, T, Options>
where
    T: Sync,
    Options: ArenaOptions<T>,
{
}

// SAFETY: This type has no `&self` methods that access shared data or fields
// with non-`Sync` interior mutability, but `T` must be `Sync` to match the
// `Send` impl, since this type implements `Clone`, effectively allowing it to
// be sent.
unsafe impl<T, Options> Sync for Iter<'_, T, Options>
where
    T: Sync,
    Options: ArenaOptions<T>,
{
}

impl<'a, T, Options> IntoIterator for &'a ManuallyDropArena<T, Options>
where
    Options: ArenaOptions<T, Mutable = Bool<false>>,
{
    type IntoIter = Iter<'a, T, Options>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A mutable iterator over the items in an arena.
pub struct IterMut<'a, T, Options: ArenaOptions<T>> {
    pub(super) inner: IterPtr<T, Options>,
    pub(super) phantom: PhantomData<&'a mut T>,
}

impl<T, Options> IterMut<'_, T, Options>
where
    Options: ArenaOptions<T, SupportsPositions = Bool<true>>,
{
    /// Converts this iterator into a [`Position`]. The position can later be
    /// turned back into an iterator with [`Arena::iter_mut_at`] (and similar
    /// methods).
    ///
    /// [`Arena::iter_mut_at`]: crate::arena::Arena::iter_mut_at
    pub fn as_position(&self) -> Position {
        self.inner.as_position()
    }
}

impl<'a, T, Options: ArenaOptions<T>> Iterator for IterMut<'a, T, Options> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `IterPtr` always returns initialized, properly aligned
        // pointers.
        Some(unsafe { self.inner.next()?.as_mut() })
    }
}

impl<T, Options: ArenaOptions<T>> FusedIterator for IterMut<'_, T, Options> {}

// SAFETY: This type yields mutable references to items in the arena, so it
// can be `Send` as long as `T` is `Send`. `T` doesn't need to be `Sync`
// because no other iterator that yields items from the arena can exist at the
// same time as this iterator.
unsafe impl<T, Options> Send for IterMut<'_, T, Options>
where
    T: Send,
    Options: ArenaOptions<T>,
{
}

// SAFETY: This type has no `&self` methods that access shared data or fields
// with non-`Sync` interior mutability.
unsafe impl<T, Options: ArenaOptions<T>> Sync for IterMut<'_, T, Options> {}

/// An owning iterator over the items in an arena.
pub struct IntoIter<T, Options: ArenaOptions<T>>(
    pub(super) IterPtr<T, Options, true>,
);

impl<T, Options: ArenaOptions<T>> Iterator for IntoIter<T, Options> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: `IterPtr` yields initialized, properly aligned pointers.
        Some(unsafe { self.0.next()?.as_ptr().read() })
    }
}

impl<T, Options: ArenaOptions<T>> FusedIterator for IntoIter<T, Options> {}

// SAFETY: This type owns the items in the arena, so it can be `Send` as long
// as `T` is `Send`.
unsafe impl<T, Options> Send for IntoIter<T, Options>
where
    T: Send,
    Options: ArenaOptions<T>,
{
}

// SAFETY: This type has no `&self` methods that access any fields.
unsafe impl<T, Options: ArenaOptions<T>> Sync for IntoIter<T, Options> {}
