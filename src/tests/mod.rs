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

mod arena;
mod manually_drop;

/// The example from the crate documentation. It's duplicated here because Miri
/// currently doesn't run doctests.
#[test]
fn crate_example() {
    use crate::Arena;
    use typenum::U64;

    struct Item(u64);

    let arena = Arena::<_, U64>::new();
    let item1 = arena.alloc(Item(1));
    let item2 = arena.alloc(Item(2));
    item1.0 += item2.0;

    assert_eq!(item1.0, 3);
    assert_eq!(item2.0, 2);
}
