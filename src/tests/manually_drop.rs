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

use crate::ManuallyDropArena;
use alloc::rc::Rc;
use core::cell::Cell;

#[test]
fn empty() {
    ManuallyDropArena::<u8, 16>::new();
}

#[test]
fn basic() {
    let mut arena = ManuallyDropArena::<_, 16>::new();
    let item1 = arena.alloc(1_u8);
    let item2 = arena.alloc(2_u8);
    let item3 = arena.alloc(3_u8);
    assert_eq!(*item1, 1_u8);
    assert_eq!(*item2, 2_u8);
    assert_eq!(*item3, 3_u8);
    unsafe {
        arena.drop();
    }
}

#[test]
fn multiple_chunks() {
    let mut arena = ManuallyDropArena::<_, 2>::new();
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
        let _ = arena.alloc(Item {
            drop_flag: flag,
        });
    }

    assert!(!drop_flags.iter().any(|f| f.get()));
    unsafe {
        arena.drop();
    }
    assert!(drop_flags.iter().all(|f| f.get()));
}

#[test]
/// Note: This test causes Miri to report a memory leak.
fn ensure_leaked() {
    struct Item(u8);

    impl Drop for Item {
        fn drop(&mut self) {
            panic!("erroneously dropped: {}", self.0);
        }
    }

    let mut arena = ManuallyDropArena::<_, 4>::new();
    for i in 0..12 {
        let _ = arena.alloc(Item(i));
    }
}
