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

#![no_std]
#![cfg_attr(feature = "dropck_eyepatch", feature(dropck_eyepatch))]
#![deny(unsafe_op_in_unsafe_fn)]
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
//! struct Item(u64);
//!
//! let arena = Arena::<_, 128>::new();
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
//! Items allocated by an [`Arena`] can contain references with the same life
//! as the arena itself, including references to other items, but the crate
//! feature `dropck_eyepatch` must be enabled. This requires Rust nightly, as
//! fixed-typed-arena must use the [eponymous unstable language feature][drop].
//!
//! [drop]: https://github.com/rust-lang/rust/issues/34761
//!
//! Alternatively, you may be able to use a [`ManuallyDropArena`] instead.
//!
//! ManuallyDropArena
//! -----------------
//!
//! This crate also provides [`ManuallyDropArena`], which is like [`Arena`] but
//! returns references of any lifetime, including `'static`. The advantage of
//! this type is that it can be used without being borrowed, but it comes with
//! the tradeoff that it will leak memory unless the unsafe [`drop`] method is
//! called.
//!
//! Iteration
//! ---------
//!
//! fixed-typed-arena’s arena types allow iteration over all allocated items.
//! Safe mutable iteration is provided for [`Arena`], and safe immutable
//! iteration is provided for all arena types if [`Options::Mutable`] is false.
//! Unsafe mutable and immutable iteration is provided for all arena types
//! regardless of options.
//!
//! [`Arena`]: arena::Arena
//! [`ManuallyDropArena`]: manually_drop::ManuallyDropArena
//! [`drop`]: manually_drop::ManuallyDropArena::drop

extern crate alloc;

mod chunk;
mod options;
#[cfg(test)]
mod tests;

pub mod arena;
pub mod manually_drop;
pub use options::{ArenaOptions, Options};

/// Arena iterators.
pub mod iter {
    pub use super::manually_drop::iter::*;
}

#[rustfmt::skip]
/// Convenience alias for [`arena::Arena`].
pub type Arena<
    T,
    const CHUNK_SIZE: usize = 16,
    const SUPPORTS_POSITIONS: bool = false,
    const MUTABLE: bool = true,
> = arena::Arena<
    T,
    Options<
        CHUNK_SIZE,
        SUPPORTS_POSITIONS,
        MUTABLE,
    >,
>;

#[rustfmt::skip]
/// Convenience alias for [`manually_drop::ManuallyDropArena`].
pub type ManuallyDropArena<
    T,
    const CHUNK_SIZE: usize = 16,
    const SUPPORTS_POSITIONS: bool = false,
    const MUTABLE: bool = true,
> = manually_drop::ManuallyDropArena<
    T,
    Options<
        CHUNK_SIZE,
        SUPPORTS_POSITIONS,
        MUTABLE,
    >,
>;
