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

use fixed_typed_arena::manually_drop;
use fixed_typed_arena::{ArenaOptions, ManuallyDropArena};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn empty() {
    let arena = ManuallyDropArena::<u8>::new();
    assert_eq!(arena.len(), 0);
    assert!(arena.is_empty());
}

#[test]
fn basic() {
    let mut arena = ManuallyDropArena::<_>::new();
    let item1: &'static mut u8 = arena.alloc(1_u8);
    let item2 = arena.alloc(2_u8);
    let item3 = arena.alloc(3_u8);

    assert_eq!(*item1, 1_u8);
    assert_eq!(*item2, 2_u8);
    assert_eq!(*item3, 3_u8);
    assert_eq!(arena.len(), 3);
    assert!(!arena.is_empty());
    unsafe {
        arena.drop();
    }
}

#[test]
fn multiple_chunks() {
    let mut arena = ManuallyDropArena::<_, 3>::new();
    let items: Vec<_> = (0..15_u8).map(|i| arena.alloc(i)).collect();
    let _: &&'static mut u8 = &items[0];

    assert!(items.into_iter().map(|n| *n).eq(0..15));
    assert_eq!(arena.len(), 15);
    unsafe {
        arena.drop();
    }
}

#[test]
fn iter() {
    type Arena<T> = ManuallyDropArena<
        T,
        /* CHUNK_SIZE */ 5,
        /* SUPPORTS_POSITIONS */ false,
        /* MUTABLE */ false,
    >;

    let mut arena = Arena::new();
    for i in 0..32_u8 {
        arena.alloc_shared(i);
    }

    assert!(arena.iter().copied().eq(0..32));
    unsafe {
        arena.drop();
    }
}

#[test]
fn ensure_dropped() {
    struct Item {
        drop_flag: Rc<Cell<bool>>,
    }

    impl Drop for Item {
        fn drop(&mut self) {
            assert!(!self.drop_flag.get(), "value dropped twice");
            self.drop_flag.set(true);
        }
    }

    let drop_flags: [Rc<Cell<bool>>; 12] = Default::default();
    let mut arena = ManuallyDropArena::<_, 4>::new();

    for flag in drop_flags.iter().cloned() {
        arena.alloc(Item {
            drop_flag: flag,
        });
    }

    assert!(drop_flags.iter().all(|f| !f.get()));
    unsafe {
        arena.drop();
    }
    assert!(drop_flags.iter().all(|f| f.get()));
}

#[test]
#[cfg_attr(miri, ignore = "intentionally leaks memory")]
fn ensure_leaked() {
    struct Item(u8);

    impl Drop for Item {
        fn drop(&mut self) {
            panic!("erroneously dropped: {}", self.0);
        }
    }

    let mut arena = ManuallyDropArena::<_, 4>::new();
    for i in 0..12 {
        arena.alloc(Item(i));
    }
}

#[test]
fn reuse() {
    let mut arena = ManuallyDropArena::<_, 3>::new();
    for i in 0..15_u8 {
        arena.alloc(i);
    }
    unsafe {
        arena.drop();
    }
    for i in 50..60_u8 {
        arena.alloc(i);
    }
    unsafe {
        arena.drop();
    }
}

struct DropArena<T, Options: ArenaOptions<T>>(
    manually_drop::ManuallyDropArena<T, Options>,
);

impl<T, Options: ArenaOptions<T>> Drop for DropArena<T, Options> {
    fn drop(&mut self) {
        unsafe {
            self.0.drop();
        }
    }
}

#[test]
#[should_panic]
fn bad_position() {
    type Arena1<T> = ManuallyDropArena<
        T,
        /* CHUNK_SIZE */ 5,
        /* SUPPORTS_POSITIONS */ true,
        /* MUTABLE */ false,
    >;

    type Arena2<T> = ManuallyDropArena<
        T,
        /* CHUNK_SIZE */ 4,
        /* SUPPORTS_POSITIONS */ true,
        /* MUTABLE */ false,
    >;

    let mut arena = Arena1::new();
    for i in 0..8_u8 {
        arena.alloc_shared(i);
    }

    let mut iter = arena.iter();
    iter.nth(3);
    let pos = iter.as_position();

    let _drop = DropArena(arena);
    let mut arena = Arena2::new();
    for i in 0..8_u8 {
        arena.alloc_shared(i);
    }

    let drop = DropArena(arena);
    drop.0.iter_at(&pos);
}

#[test]
#[should_panic]
fn bad_position_reused_arena() {
    type Arena<T> = ManuallyDropArena<
        T,
        /* CHUNK_SIZE */ 5,
        /* SUPPORTS_POSITIONS */ true,
        /* MUTABLE */ false,
    >;

    let mut arena = Arena::new();
    for i in 0..8_u8 {
        arena.alloc_shared(i);
    }

    let mut iter = arena.iter();
    iter.nth(3);
    let pos = iter.as_position();

    unsafe {
        arena.drop();
    }
    for i in 0..8_u8 {
        arena.alloc_shared(i);
    }

    let drop = DropArena(arena);
    drop.0.iter_at(&pos);
}
