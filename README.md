fixed-typed-arena
=================

An arena that allocates values of a single type (similar to [typed-arena])
using chunks of memory that have a configurable fixed size. This enables it
to perform allocations in non-amortized O(1) (constant) time.

Other arena implementations, like [typed-arena], are optimized for
throughput: they allocate chunks of memory with exponentially increasing
sizes, which results in *amortized* constant-time allocations.

[typed-arena]: https://docs.rs/typed-arena

**fixed-typed-arena** is optimized for latency: it allocates chunks of
memory with a fixed, configurable size, and individual value allocations
are performed in non-amortized constant time.

This crate depends only on [`core`] and [`alloc`], so it can be used in
`no_std` environments that support [`alloc`].

[`core`]: https://doc.rust-lang.org/core/
[`alloc`]: https://doc.rust-lang.org/alloc/

Example
-------

```rust
use fixed_typed_arena::Arena;
struct Item(u64);

let arena = Arena::<_, 128>::new();
let item1 = arena.alloc(Item(1));
let item2 = arena.alloc(Item(2));
item1.0 += item2.0;

assert_eq!(item1.0, 3);
assert_eq!(item2.0, 2);
```

References
----------

Items allocated by an [`Arena`] can contain references with the same life
as the arena itself, including references to other items, but the crate
feature `dropck_eyepatch` must be enabled. This requires Rust nightly, as
fixed-typed-arena must use the [eponymous unstable language feature][drop].

[drop]: https://github.com/rust-lang/rust/issues/34761

Alternatively, you may be able to use a [`ManuallyDropArena`] instead.

ManuallyDropArena
-----------------

This crate also provides [`ManuallyDropArena`], which is like [`Arena`] but
returns references of any lifetime, including `'static`. The advantage of
this type is that it can be used without being borrowed, but it comes with
the tradeoff that it will leak memory unless the unsafe [`drop`] method is
called.

Iteration
---------

fixed-typed-arena’s arena types allow iteration over all allocated items.
Safe mutable iteration is provided for [`Arena`], and safe immutable
iteration is provided for all arena types if [`Options::Mutable`] is false.
Unsafe mutable and immutable iteration is provided for all arena types
regardless of options.

[`Arena`]: https://docs.rs/fixed-typed-arena/0.3/fixed_typed_arena/arena/struct.Arena.html
[`ManuallyDropArena`]: https://docs.rs/fixed-typed-arena/0.3/fixed_typed_arena/manually_drop/struct.ManuallyDropArena.html
[`drop`]: https://docs.rs/fixed-typed-arena/0.3/fixed_typed_arena/manually_drop/struct.ManuallyDropArena.html#method.drop
[`Options::Mutable`]: https://docs.rs/fixed-typed-arena/0.3/fixed_typed_arena/struct.Options.html#associatedtype.Mutable

Documentation
-------------

[Documentation is available on docs.rs.](https://docs.rs/fixed-typed-arena)

License
-------

fixed-typed-arena is licensed under version 3 of the GNU General Public
License, or (at your option) any later version. See [LICENSE](LICENSE).

Contributing
------------

By contributing to fixed-typed-arena, you agree that your contribution may be
used according to the terms of fixed-typed-arena’s license.
