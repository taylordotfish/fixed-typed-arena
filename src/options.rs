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

use alloc::sync::Arc;
use core::convert::Infallible;
use core::mem::MaybeUninit;

pub struct Usize<const N: usize>(());
pub struct Bool<const B: bool>(());

pub trait ChunkSize<T> {
    type Array;
}

pub trait SupportsPositions {
    type Rc: Clone + Send + Sync;
    fn init_rc(_rc: &mut Option<Self::Rc>) {}
}

impl<T, const N: usize> ChunkSize<T> for Usize<N> {
    type Array = [MaybeUninit<T>; N];
}

impl SupportsPositions for Bool<false> {
    type Rc = Infallible;
}

impl SupportsPositions for Bool<true> {
    type Rc = Arc<()>;

    fn init_rc(rc: &mut Option<Self::Rc>) {
        rc.get_or_insert_with(Arc::default);
    }
}

mod sealed {
    pub trait Sealed {}
}

/// Arena options trait.
///
/// This is a sealed trait; use the [`Options`] type, which implements this
/// trait.
pub trait ArenaOptions<T>: sealed::Sealed {
    /// The number of elements of type `T` that each chunk can hold.
    type ChunkSize: ChunkSize<T>;

    /// If true, enables the use of [`Position`]s, allowing methods like
    /// [`IterMut::as_position`] and [`Arena::iter_mut_at`] to be called, at
    /// the cost of using slightly more memory.
    ///
    /// [`Position`]: crate::iter::Position
    /// [`IterMut::as_position`]: crate::iter::IterMut::as_position
    /// [`Arena::iter_mut_at`]: crate::arena::Arena::iter_mut_at
    type SupportsPositions: SupportsPositions;

    /// If true, the arena is able to return mutable references.
    type Mutable;
}

/// Arena options.
///
/// Const parameters correspond to associated types in [`ArenaOptions`] as
/// follows; see those associated types for documentation:
///
/// * `CHUNK_SIZE`: [`ArenaOptions::ChunkSize`]
/// * `SUPPORTS_POSITIONS`: [`ArenaOptions::SupportsPositions`]
/// * `MUTABLE`: [`ArenaOptions::Mutable`]
pub struct Options<
    const CHUNK_SIZE: usize = 16,
    const SUPPORTS_POSITIONS: bool = false,
    const MUTABLE: bool = true,
>;

#[rustfmt::skip]
impl<
    const CHUNK_SIZE: usize,
    const SUPPORTS_POSITIONS: bool,
    const MUTABLE: bool,
> sealed::Sealed for Options<
    CHUNK_SIZE,
    SUPPORTS_POSITIONS,
    MUTABLE,
> {}

#[rustfmt::skip]
impl<
    T,
    const CHUNK_SIZE: usize,
    const SUPPORTS_POSITIONS: bool,
    const MUTABLE: bool,
> ArenaOptions<T> for Options<
    CHUNK_SIZE,
    SUPPORTS_POSITIONS,
    MUTABLE,
> where
    Bool<SUPPORTS_POSITIONS>: SupportsPositions,
{
    type ChunkSize = Usize<CHUNK_SIZE>;
    type SupportsPositions = Bool<SUPPORTS_POSITIONS>;
    type Mutable = Bool<MUTABLE>;
}
