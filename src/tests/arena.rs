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

use crate::Arena;
use core::cell::Cell;
use typenum::{U0, U2, U4};

#[test]
fn empty() {
    Arena::<u8>::new();
}

#[test]
fn basic() {
    let arena = Arena::<_>::new();
    let item1 = arena.alloc(1_u8);
    let item2 = arena.alloc(2_u8);
    let item3 = arena.alloc(3_u8);
    assert_eq!(*item1, 1_u8);
    assert_eq!(*item2, 2_u8);
    assert_eq!(*item3, 3_u8);
}

#[test]
fn multiple_chunks() {
    let arena = Arena::<_, U2>::new();
    let item1 = arena.alloc(1_u8);
    let item2 = arena.alloc(2_u8);
    let item3 = arena.alloc(3_u8);
    let item4 = arena.alloc(4_u8);
    let item5 = arena.alloc(5_u8);
    assert_eq!(*item1, 1_u8);
    assert_eq!(*item2, 2_u8);
    assert_eq!(*item3, 3_u8);
    assert_eq!(*item4, 4_u8);
    assert_eq!(*item5, 5_u8);
}

#[test]
fn ensure_dropped() {
    struct Item<'a> {
        drop_flag: &'a Cell<bool>,
    }

    impl Drop for Item<'_> {
        fn drop(&mut self) {
            assert!(!self.drop_flag.get(), "value dropped twice");
            self.drop_flag.set(true);
        }
    }

    let drop_flags: [Cell<bool>; 32] = Default::default();
    let arena = Arena::<_, U4>::new();

    for flag in &drop_flags {
        let _ = arena.alloc(Item {
            drop_flag: flag,
        });
    }

    assert!(!drop_flags.iter().all(Cell::get));
    core::mem::drop(arena);
    assert!(drop_flags.iter().all(Cell::get));
}

#[cfg(feature = "dropck_eyepatch")]
#[test]
fn same_life_ref() {
    struct Item<'a> {
        next: Cell<Option<&'a Self>>,
    }

    let arena = Arena::<_>::new();
    let item1 = arena.alloc(Item {
        next: Cell::new(None),
    });
    let item2 = arena.alloc(Item {
        next: Cell::new(Some(item1)),
    });
    item1.next.set(Some(item2));
}

#[test]
#[should_panic]
fn zero_chunk_size() {
    let arena = Arena::<_, U0>::new();
    let _ = arena.alloc(0_u8);
}
