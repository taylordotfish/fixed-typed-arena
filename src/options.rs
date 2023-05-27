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

//! Arena options.

use alloc::sync::Arc;
use core::convert::Infallible;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

/// Represents a [`usize`].
pub struct Usize<const N: usize>(());

/// Represents a [`bool`].
pub struct Bool<const B: bool>(());

mod detail {
    pub trait ChunkSizePriv<T> {
        type Array;
    }

    pub trait SupportsPositionsPriv {
        type Rc: Clone + Send + Sync;
        fn init_rc(_rc: &mut Option<Self::Rc>) {}
    }

    pub trait MutablePriv {}
}

pub(crate) use detail::*;

/// Trait bound on [`ArenaOptions::ChunkSize`].
pub trait ChunkSize<T>: ChunkSizePriv<T> {}

impl<T, const N: usize> ChunkSize<T> for Usize<N> {}

impl<T, const N: usize> ChunkSizePriv<T> for Usize<N> {
    type Array = [MaybeUninit<T>; N];
}

/// Trait bound on [`ArenaOptions::SupportsPositions`].
pub trait SupportsPositions: SupportsPositionsPriv {}

impl SupportsPositions for Bool<false> {}
impl SupportsPositions for Bool<true> {}

impl SupportsPositionsPriv for Bool<false> {
    type Rc = Infallible;
}

impl SupportsPositionsPriv for Bool<true> {
    type Rc = Arc<()>;

    fn init_rc(rc: &mut Option<Self::Rc>) {
        rc.get_or_insert_with(Arc::default);
    }
}

/// Trait bound on [`ArenaOptions::Mutable`].
pub trait Mutable: MutablePriv {}

impl Mutable for Bool<false> {}
impl Mutable for Bool<true> {}
impl<const B: bool> MutablePriv for Bool<B> {}

mod sealed {
    pub trait Sealed {}
}

/// Arena options trait.
///
/// This is a sealed trait; use the [`Options`] type, which implements this
/// trait.
pub trait ArenaOptions<T>: sealed::Sealed {
    /// The number of elements of type `T` that each chunk can hold.
    ///
    /// *Default:* 16
    type ChunkSize: ChunkSize<T>;

    /// If true, enables the use of [`Position`]s, allowing methods like
    /// [`IterMut::as_position`] and [`Arena::iter_mut_at`] to be called, at
    /// the cost of using slightly more memory.
    ///
    /// *Default:* false
    ///
    /// [`Position`]: crate::iter::Position
    /// [`IterMut::as_position`]: crate::iter::IterMut::as_position
    /// [`Arena::iter_mut_at`]: crate::arena::Arena::iter_mut_at
    type SupportsPositions: SupportsPositions;

    /// If true, the arena is able to return mutable references.
    ///
    /// *Default:* true
    type Mutable: Mutable;
}

/// Arena options.
///
/// This type implements [`ArenaOptions`]. Const parameters correspond to
/// associated types in [`ArenaOptions`] as follows; see those associated types
/// for documentation:
///
/// Const parameter      | Associated type
/// -------------------- | -----------------------------------
/// `CHUNK_SIZE`         | [`ArenaOptions::ChunkSize`]
/// `SUPPORTS_POSITIONS` | [`ArenaOptions::SupportsPositions`]
/// `MUTABLE`            | [`ArenaOptions::Mutable`]
#[rustfmt::skip]
pub type Options<
    const CHUNK_SIZE: usize = 16,
    const SUPPORTS_POSITIONS: bool = false,
    const MUTABLE: bool = true,
> = TypedOptions<
    Usize<CHUNK_SIZE>,
    Bool<SUPPORTS_POSITIONS>,
    Bool<MUTABLE>,
>;

/// Like [`Options`], but uses types instead of const parameters.
///
/// [`Options`] is actually a type alias of this type.
#[allow(clippy::type_complexity)]
#[rustfmt::skip]
pub struct TypedOptions<
    ChunkSize = Usize<16>,
    SupportsPositions = Bool<false>,
    Mutable = Bool<true>,
>(PhantomData<fn() -> (
    ChunkSize,
    SupportsPositions,
    Mutable,
)>);

#[rustfmt::skip]
impl<
    ChunkSize,
    SupportsPositions,
    Mutable,
> sealed::Sealed for TypedOptions<
    ChunkSize,
    SupportsPositions,
    Mutable,
> {}

#[rustfmt::skip]
impl<
    T,
    ChunkSize: self::ChunkSize<T>,
    SupportsPositions: self::SupportsPositions,
    Mutable: self::Mutable,
> ArenaOptions<T> for TypedOptions<
    ChunkSize,
    SupportsPositions,
    Mutable,
> {
    type ChunkSize = ChunkSize;
    type SupportsPositions = SupportsPositions;
    type Mutable = Mutable;
}
