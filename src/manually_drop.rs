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

//! An arena that returns references with arbitrary lifetimes.

use super::chunk::ChunkRef;
use super::options::{Bool, ChunkSizePriv, SupportsPositionsPriv};
use super::ArenaOptions;
use alloc::alloc::handle_alloc_error;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::fmt::{Debug, Display};
use core::hint::unreachable_unchecked;
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};

pub(crate) mod iter;
use iter::{IntoIter, Iter, IterMut, IterPtr, Position};

type Array<T, Options> =
    <<Options as ArenaOptions<T>>::ChunkSize as ChunkSizePriv<T>>::Array;
type SupportsPositions<T, Options> =
    <Options as ArenaOptions<T>>::SupportsPositions;
type ArenaRc<T, Options> =
    <SupportsPositions<T, Options> as SupportsPositionsPriv>::Rc;
type ArenaChunk<T, Options> = ChunkRef<T, Array<T, Options>>;

/// Checks whether `old` and `new` point to the same allocation (see
/// [`Arc::ptr_eq`]), but allows `old` to be [`None`], even if `new` is
/// [`Some`].
fn valid_rc_update<T>(old: &Option<Arc<T>>, new: &Option<Arc<T>>) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => Arc::ptr_eq(old, new),
        (Some(_old), None) => false,
        (None, _new) => true,
    }
}

// Invariants:
//
// * Every chunk except for `tail` must be full (all items initialized).
// * If `tail_len` is less than `Self::CHUNK_SIZE`, `tail` is `Some`.
// * If `tail` is `Some`, it contains at least one item (`tail_len > 0`).
// * If `tail` is `Some`, the items in `tail` up to index `tail_len`
//   (exclusive) are initialized.
//
/// Like [`Arena`], but returns references of any lifetime, including
/// `'static`.
///
/// This lets the arena be used without being borrowed, but it comes with the
/// tradeoff that the arena leaks memory unless the unsafe [`drop`](Self::drop)
/// method is called.
///
/// [`Arena`]: super::arena::Arena
pub struct ManuallyDropArena<T, Options: ArenaOptions<T> = super::Options> {
    rc: Option<ArenaRc<T, Options>>,
    head: Option<ArenaChunk<T, Options>>,
    tail: Option<ArenaChunk<T, Options>>,
    tail_len: usize,
    len: usize,
    /// Lets dropck know that `T` may be dropped.
    phantom: PhantomData<Box<T>>,
}

impl<T, Options: ArenaOptions<T>> Default for ManuallyDropArena<T, Options> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, Options: ArenaOptions<T>> ManuallyDropArena<T, Options> {
    const CHUNK_SIZE: usize = ArenaChunk::<T, Options>::CAPACITY;

    /// Creates a new [`ManuallyDropArena`].
    pub fn new() -> Self {
        Self {
            rc: None,
            head: None,
            tail: None,
            tail_len: Self::CHUNK_SIZE,
            len: 0,
            phantom: PhantomData,
        }
    }

    fn ensure_free_space(&mut self) -> Result<(), impl Debug + Display> {
        assert!(
            Self::CHUNK_SIZE > 0,
            "cannot allocate items when chunk size is 0",
        );
        if self.tail_len < Self::CHUNK_SIZE {
            // `self.tail` cannot be `None`. The only time `self.tail` is
            // `None` is after calling `Self::new`, which also sets
            // `self.tail_len` to `Self::CHUNK_SIZE`.
            return Ok(());
        }

        let chunk = if let Some(chunk) = ChunkRef::new(self.tail.take()) {
            chunk
        } else {
            return Err("could not allocate chunk");
        };

        self.head.get_or_insert_with(|| chunk.clone());
        self.tail = Some(chunk);
        self.tail_len = 0;
        Ok(())
    }

    fn alloc_ptr(&mut self, value: T) -> NonNull<T> {
        self.try_alloc_ptr(value).unwrap_or_else(|| {
            handle_alloc_error(ArenaChunk::<T, Options>::LAYOUT);
        })
    }

    fn try_alloc_ptr(&mut self, value: T) -> Option<NonNull<T>> {
        self.ensure_free_space().ok()?;
        SupportsPositions::<T, Options>::init_rc(&mut self.rc);

        let chunk = self.tail.as_mut().unwrap_or_else(|| {
            // SAFETY: `Self::ensure_free_space` ensures that `self.tail`
            // is not `None`.
            unsafe { unreachable_unchecked() }
        });

        // SAFETY: `Self::ensure_free_space` ensures that `self.tail_len` is
        // less than the chunk size.
        let item = unsafe { chunk.get(self.tail_len) };

        // SAFETY: `ChunkRef::get` returns valid, properly aligned pointers.
        unsafe {
            item.as_ptr().write(value);
        }

        self.tail_len += 1;
        self.len += 1;
        Some(item)
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
    /// * `T`'s [`Drop`] impl does not directly or indirectly access any data
    ///   via the references to other items or parts of items. (This is
    ///   essentially the requirement imposed by [`#[may_dangle]`][dropck].)
    ///
    /// Additionally, there must be no instances of [`Iter`] or [`IterMut`]
    /// for this arena.
    ///
    /// [dropck]: https://doc.rust-lang.org/nomicon/dropck.html
    pub unsafe fn drop(&mut self) {
        let mut head = if let Some(head) = self.head.take() {
            head
        } else {
            return;
        };

        self.tail = None;
        let tail_len = mem::replace(&mut self.tail_len, Self::CHUNK_SIZE);
        self.len = 0;
        self.rc = None;

        // Drop the items in all chunks except the last.
        while let Some(next) = head.next() {
            // SAFETY: All chunks except for the tail are guaranteed to
            // be full (all items initialized). We know this isn't the
            // tail chunk because `head.next()` is not `None`.
            unsafe {
                head.drop_all();
            }

            // SAFETY: No clones of this `ChunkRef` exist. `self.head`
            // and `self.tail` are both `None`, and the chunks form a
            // singly linked list. Caller guarantees no iterators exist.
            unsafe {
                head.dealloc();
            }
            head = next;
        }

        // `head` is now the tail chunk; drop its items.
        for i in 0..tail_len {
            // SAFETY: The items in the tail chunk (when not `None`) at
            // indices up to `self.tail_len` are always initialized.
            unsafe {
                head.drop_item(i);
            }
        }

        // SAFETY: No clones of this `ChunkRef` exist for the same
        // reasons as the other chunks above.
        unsafe {
            head.dealloc();
        }
    }

    /// Alias of [`Self::drop`]. Can be used to prevent name collisions when
    /// this arena is stored in a [`Deref`](core::ops::Deref) type:
    ///
    /// ```
    /// # use fixed_typed_arena::ManuallyDropArena;
    /// let mut arena = Box::new(ManuallyDropArena::<u8, 8>::new());
    /// //unsafe { arena.drop() }; // Compile error: resolves to `Drop::drop`
    /// unsafe { arena.manually_drop() }; // Works as expected
    /// ```
    ///
    /// # Safety
    ///
    /// Same requirements as [`Self::drop`].
    pub unsafe fn manually_drop(&mut self) {
        // SAFETY: Checked by caller.
        unsafe {
            self.drop();
        }
    }

    /// Returns the total number of items that have been allocated.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Checks whether the arena is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a reference to the allocated item. The reference can have any
    /// lifetime, including `'static`, as long as `T` outlives that lifetime.
    ///
    /// This method calls [`handle_alloc_error`] if memory allocation fails;
    /// for a version that returns [`None`], see [`Self::try_alloc`].
    ///
    /// [`handle_alloc_error`]: alloc::alloc::handle_alloc_error
    pub fn alloc<'a>(&mut self, value: T) -> &'a mut T
    where
        Options: 'a + ArenaOptions<T, Mutable = Bool<true>>,
    {
        // SAFETY: `Self::alloc_ptr` returns initialized, properly aligned
        // pointers, and we can return a reference with an arbitrary lifetime
        // because the arena must be manually dropped.
        unsafe { self.alloc_ptr(value).as_mut() }
    }

    /// Like [`Self::alloc`], but returns [`None`] if memory allocation fails.
    pub fn try_alloc<'a>(&mut self, value: T) -> Option<&'a mut T>
    where
        Options: 'a + ArenaOptions<T, Mutable = Bool<true>>,
    {
        // SAFETY: See `Self::alloc`.
        Some(unsafe { self.try_alloc_ptr(value)?.as_mut() })
    }

    /// Allocates a new item in the arena and initializes it with `value`.
    /// Returns a shared/immutable reference to the allocated item. The
    /// reference can have any lifetime, including `'static`, as long as `T`
    /// outlives that lifetime.
    ///
    /// This method calls [`handle_alloc_error`] if memory allocation fails;
    /// for a version that returns [`None`], see [`Self::try_alloc`].
    ///
    /// [`handle_alloc_error`]: alloc::alloc::handle_alloc_error
    pub fn alloc_shared<'a>(&mut self, value: T) -> &'a T
    where
        Options: 'a,
    {
        // SAFETY: See `Self::alloc`.
        unsafe { self.alloc_ptr(value).as_ref() }
    }

    /// Like [`Self::alloc_shared`], but returns [`None`] if memory allocation
    /// fails.
    pub fn try_alloc_shared<'a>(&mut self, value: T) -> Option<&'a T>
    where
        Options: 'a,
    {
        // SAFETY: See `Self::alloc`.
        Some(unsafe { self.try_alloc_ptr(value)?.as_ref() })
    }

    fn end(&self) -> *const T {
        self.tail.as_ref().map_or(ptr::null(), |c| {
            // SAFETY: `self.tail_len` is necessarily less than or equal to
            // the chunk capacity.
            unsafe { c.get(self.tail_len) }.as_ptr()
        })
    }

    fn iter_ptr<const DROP: bool>(&self) -> IterPtr<T, Options, DROP> {
        IterPtr {
            chunk: self.head.clone(),
            index: 0,
            end: self.end(),
            rc: self.rc.clone(),
            phantom: PhantomData,
        }
    }

    /// Returns an iterator over the items in this arena.
    pub fn iter<'a>(&self) -> Iter<'a, T, Options>
    where
        Options: ArenaOptions<T, Mutable = Bool<false>>,
    {
        unsafe { self.iter_unchecked() }
    }

    /// Returns an iterator over the items in this arena.
    ///
    /// # Safety
    ///
    /// There must be no mutable references (or references derived from mutable
    /// references) to items (or parts of items) in this arena or instances of
    /// [`IterMut`] for this arena.
    pub unsafe fn iter_unchecked<'a>(&self) -> Iter<'a, T, Options> {
        Iter {
            inner: self.iter_ptr::<false>(),
            phantom: PhantomData,
        }
    }

    /// Returns a mutable iterator over the items in this arena.
    ///
    /// # Safety
    ///
    /// There must be no references to items (or parts of items) in this arena
    /// or instances of [`Iter`] or [`IterMut`] for this arena.
    pub unsafe fn iter_mut_unchecked<'a>(
        &mut self,
    ) -> IterMut<'a, T, Options> {
        IterMut {
            inner: self.iter_ptr::<false>(),
            phantom: PhantomData,
        }
    }

    /// Returns an owning iterator over the items in this arena.
    ///
    /// # Safety
    ///
    /// There must be no references to items (or parts of items) in this arena
    /// or instances of [`Iter`] or [`IterMut`] for this arena.
    pub unsafe fn into_iter_unchecked(self) -> IntoIter<T, Options> {
        IntoIter(self.iter_ptr())
    }
}

impl<T, Options> ManuallyDropArena<T, Options>
where
    Options: ArenaOptions<T, SupportsPositions = Bool<true>>,
{
    fn iter_ptr_at(&self, position: &Position) -> IterPtr<T, Options> {
        assert!(
            valid_rc_update(&position.rc, &self.rc),
            "`position` is not part of this arena",
        );

        // SAFETY: Checking the pointer equality of `self.rc` and `position.rc`
        // above ensures `position` belongs to this arena. Note that if
        // `position.rc` is `None`, it may have come from a different arena,
        // but this is okay, because in this case `position` does not contain a
        // pointer that we dereference.
        let chunk = position.chunk.map(|p| unsafe { ChunkRef::from_ptr(p) });

        IterPtr {
            chunk: chunk.or_else(|| self.head.clone()),
            index: position.index,
            end: self.end(),
            rc: self.rc.clone(),
            phantom: PhantomData,
        }
    }

    /// Returns an iterator starting at the specified position.
    ///
    /// # Panics
    ///
    /// May panic if `position` does not refer to a position in this arena.
    pub fn iter_at<'a>(&self, position: &Position) -> Iter<'a, T, Options>
    where
        Options: ArenaOptions<T, Mutable = Bool<false>>,
    {
        unsafe { self.iter_at_unchecked(position) }
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
    pub unsafe fn iter_at_unchecked<'a>(
        &self,
        position: &Position,
    ) -> Iter<'a, T, Options> {
        Iter {
            inner: self.iter_ptr_at(position),
            phantom: PhantomData,
        }
    }

    /// Returns a mutable iterator starting at the specified position.
    ///
    /// # Panics
    ///
    /// May panic if `position` does not refer to a position in this arena.
    ///
    /// # Safety
    ///
    /// Same requirements as [`Self::iter_mut_unchecked`].
    pub unsafe fn iter_mut_at_unchecked<'a>(
        &mut self,
        position: &Position,
    ) -> IterMut<'a, T, Options> {
        IterMut {
            inner: self.iter_ptr_at(position),
            phantom: PhantomData,
        }
    }
}

// SAFETY: `ManuallyDropArena` owns its items and provides access to them using
// standard borrow rules, so it can be `Sync` as long as `T` is `Sync`.
unsafe impl<T, Options> Sync for ManuallyDropArena<T, Options>
where
    T: Sync,
    Options: ArenaOptions<T>,
{
}

// SAFETY: `ManuallyDropArena` owns its items, so it can be `Send` as long
// as `T` is both `Send` and `Sync`. `T` must be `Sync` because the life of
// the iterators `ManuallyDropArena` provides (e.g., from `Self::iter`) is
// unbounded, so a caller could obtain such an iterator and then move the arena
// to another thread while still possessing the iterator.
unsafe impl<T, Options> Send for ManuallyDropArena<T, Options>
where
    T: Send + Sync,
    Options: ArenaOptions<T>,
{
}
