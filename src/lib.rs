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

#![no_std]
#![cfg_attr(feature = "dropck_eyepatch", feature(dropck_eyepatch))]
#![cfg_attr(not(feature = "unstable"), allow(unused_unsafe))]
#![cfg_attr(
    feature = "unstable",
    feature(unsafe_block_in_unsafe_fn),
    deny(unsafe_op_in_unsafe_fn)
)]
#![warn(clippy::pedantic)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

//! An arena that allocates values of a single type (similar to [typed-arena])
//! using chunks of memory that have a configurable fixed size. This enables it
//! to perform allocations in non-amortized O(1) (constant) time.
//!
//! Other arena implementations, like [typed-arena], are optimized for
//! throughput: they allocate chunks of memory with exponentially increasing
//! sizes, which results in *amortized* constant-time allocations.
//!
//! [typed-arena]: https://docs.rs/typed-arena
//!
//! **fixed-typed-arena** is optimized for latency: it allocates chunks of
//! memory with a fixed, configurable size, and individual value allocations
//! are performed in non-amortized constant time.
//!
//! This crate depends only on [`core`] and [`alloc`], so it can be used in
//! `no_std` environments that support [`alloc`].
//!
//! [`core`]: https://doc.rust-lang.org/core/
//! [`alloc`]: https://doc.rust-lang.org/alloc/
//!
//! Example
//! -------
//!
//! ```rust
//! use fixed_typed_arena::Arena;
//! use typenum::U64;
//!
//! struct Item(u64);
//!
//! let arena = Arena::<_, U64>::new();
//! let item1 = arena.alloc(Item(1));
//! let item2 = arena.alloc(Item(2));
//! item1.0 += item2.0;
//!
//! assert_eq!(item1.0, 3);
//! assert_eq!(item2.0, 2);
//! ```
//!
//! References
//! ----------
//!
//! Items allocated by an [`Arena`] can contain references to other items in
//! the same arena, but the crate feature `"dropck_eyepatch"` must be enabled
//! (which requires Rust nightly). This is because fixed-typed-arena has to use
//! the [unstable feature of the same name][dropck_eyepatch].
//!
//! [dropck_eyepatch]: https://github.com/rust-lang/rust/issues/34761
//!
//! Alternatively, you may be able to use a [`ManuallyDropArena`] instead.
//!
//! ManuallyDropArena
//! -----------------
//!
//! This crate also provides [`ManuallyDropArena`], a type like [`Arena`] that
//! returns references of any lifetime, including `'static`. The advantage of
//! this type is that it can be used without being borrowed, but it comes with
//! the tradeoff that it will leak memory unless the unsafe [`drop`] method is
//! called.
//!
//! [`drop`]: ManuallyDropArena::drop

extern crate alloc;

mod arena;
mod chunk;
mod inner;
mod manually_drop;
#[cfg(test)]
mod tests;

pub use arena::Arena;
pub use manually_drop::ManuallyDropArena;
