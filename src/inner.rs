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

use super::chunk::Chunk;
use core::hint::unreachable_unchecked;
use core::marker::PhantomData;

// Invariants:
//
// * If `tail` is not `None`, the items in `tail` at indices up to (but not
//   including) `tail_len` are initialized.
//
// * If `tail_len` is less than `CHUNK_SIZE`, `tail` is not `None`.
//
// * Every `Chunk`, except possibly for `tail`, is full (i.e., all items in
//   that chunk are initialized).
pub struct ArenaInner<T, const CHUNK_SIZE: usize> {
    tail: Option<Chunk<T, CHUNK_SIZE>>,
    tail_len: usize,
    // Lets dropck know that `T` may be dropped.
    phantom: PhantomData<T>,
}

impl<T, const CHUNK_SIZE: usize> ArenaInner<T, CHUNK_SIZE> {
    pub fn new() -> Self {
        Self {
            tail: None,
            tail_len: CHUNK_SIZE,
            phantom: PhantomData,
        }
    }

    fn ensure_free_space(&mut self) {
        assert!(CHUNK_SIZE > 0, "cannot allocate items when chunk size is 0");
        if self.tail_len < CHUNK_SIZE {
            // `self.tail` cannot be `None`. The only time `self.tail` is
            // `None` is after calling `Self::new`, which also sets
            // `self.tail_len` to `CHUNK_SIZE`.
            return;
        }
        self.tail = Some(Chunk::new(self.tail.take()));
        self.tail_len = 0;
    }

    pub fn alloc(&mut self, value: T) -> &mut T {
        self.ensure_free_space();
        let chunk = self.tail.as_mut().unwrap_or_else(|| {
            // SAFETY: `Self::ensure_free_space` ensures that `self.tail`
            // is not `None`.
            unsafe { unreachable_unchecked() }
        });

        // SAFETY: `Self::ensure_free_space` ensures that `self.tail_len` is
        // less than the chunk size.
        let item = unsafe { chunk.get(self.tail_len) };
        self.tail_len += 1;

        // SAFETY: `Chunk::get` guarantees the pointer points to valid,
        // properly aligned memory.
        unsafe {
            item.as_ptr().write(value);
        }
        // SAFETY: We just initialized `uninit` with `value`.
        unsafe { &mut *item.as_ptr() }
    }
}

macro_rules! drop_fn {
    () => {
        fn drop(&mut self) {
            let mut tail = if let Some(tail) = self.tail.take() {
                tail
            } else {
                return;
            };

            // Drop the items in the tail chunk.
            for i in (0..self.tail_len).rev() {
                // SAFETY: The items in `self.tail` (when not `None`) at
                // indices up to `self.tail_len` are always initialized.
                unsafe {
                    tail.drop_item(i);
                }
            }

            // Drop the items in all other chunks.
            let mut prev = tail.into_prev();
            while let Some(mut chunk) = prev {
                // SAFETY: All chunks, except possibly for tail, which we
                // already handled above, are guaranteed to be full (all items
                // initialized).
                unsafe {
                    chunk.drop_all();
                }
                prev = chunk.into_prev();
            }
        }
    };
}

#[cfg(not(feature = "dropck_eyepatch"))]
impl<T, const CHUNK_SIZE: usize> Drop for ArenaInner<T, CHUNK_SIZE> {
    drop_fn!();
}

// SAFETY: This `Drop` impl does directly or indirectly access any data in any
// `T`, except for calling `T`'s destructor (see [1]), and `Self` contains a
// `PhantomData<T>` so dropck knows that `T` may be dropped (see [2]).
//
// [1]: https://doc.rust-lang.org/nomicon/dropck.html
// [2]: https://forge.rust-lang.org/libs/maintaining-std.html
//      #is-there-a-manual-drop-implementation
#[cfg(feature = "dropck_eyepatch")]
unsafe impl<#[may_dangle] T, const CHUNK_SIZE: usize> Drop
    for ArenaInner<T, CHUNK_SIZE>
{
    drop_fn!();
}
