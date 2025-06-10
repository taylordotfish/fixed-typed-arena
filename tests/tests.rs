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

use fixed_typed_arena::Arena;
use std::cell::Cell;

#[test]
fn empty() {
    let arena = Arena::<u8>::new();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
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
    assert_eq!(arena.len(), 3);
    assert!(!arena.is_empty());
}

#[test]
fn multiple_chunks() {
    let arena = Arena::<_, 4>::new();
    let items: Vec<_> = (0..20_u8).map(|i| arena.alloc(i)).collect();
    let _: &&mut u8 = &items[0];

    assert!(items.into_iter().map(|n| *n).eq(0..20));
    assert_eq!(arena.len(), 20);
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
    let arena = Arena::<_, 4>::new();

    for flag in &drop_flags {
        arena.alloc(Item {
            drop_flag: flag,
        });
    }

    assert!(drop_flags.iter().all(|f| !f.get()));
    drop(arena);
    assert!(drop_flags.iter().all(Cell::get));
}

#[cfg(feature = "dropck_eyepatch")]
#[test]
fn same_life_ref() {
    struct Item<'a> {
        next: Cell<Option<&'a Self>>,
    }

    let arena = Arena::<_, 16>::new();
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
    let arena = Arena::<_, 0>::new();
    arena.alloc(0_u8);
}

#[test]
fn iter() {
    type Arena<T> = self::Arena<
        T,
        /* CHUNK_SIZE */ 6,
        /* SUPPORTS_POSITIONS */ false,
        /* MUTABLE */ false,
    >;

    let arena = Arena::new();
    for i in 15..35_u32 {
        arena.alloc_shared(i);
    }
    assert!(arena.iter().copied().eq(15..35));
}

#[test]
fn iter_unchecked() {
    let arena = Arena::<_, 5>::new();
    for i in 0..32_u8 {
        arena.alloc(i);
    }
    assert!(unsafe { arena.iter_unchecked() }.copied().eq(0..32));
}

#[test]
fn iter_mut() {
    let mut arena = Arena::<_, 4>::new();
    for i in 0..32_u8 {
        arena.alloc(i);
    }
    assert!(arena.iter_mut().map(|n| *n).eq(0..32));
}

#[test]
fn into_iter() {
    let arena = Arena::<_, 5>::new();
    for i in 25..50_u16 {
        arena.alloc(i);
    }
    assert!(arena.into_iter().eq(25..50));
}

#[test]
fn position() {
    let mut arena = Arena::<_, 4, true>::new();
    for i in 0..32_u8 {
        arena.alloc(i);
    }

    let mut iter = arena.iter_mut();
    iter.nth(7);
    let pos1 = iter.as_position();
    iter.nth(13);
    let pos2 = iter.as_position();

    for i in 32..48 {
        arena.alloc(i);
    }

    assert_eq!(arena.len(), 48);
    assert!(arena.iter_mut_at(&pos1).map(|n| *n).eq(8..48));
    assert!(arena.iter_mut_at(&pos2).map(|n| *n).eq(22..48));
}

#[test]
#[should_panic]
fn bad_position() {
    let mut arena = Arena::<_, 4, true>::new();
    for i in 0..8_u8 {
        arena.alloc(i);
    }

    let mut iter = arena.iter_mut();
    iter.nth(3);
    let pos = iter.as_position();

    drop(arena);
    let mut arena = Arena::<_, 4, true>::new();
    for i in 0..8_u8 {
        arena.alloc(i);
    }
    arena.iter_mut_at(&pos);
}
