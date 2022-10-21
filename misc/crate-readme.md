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

Items allocated by an [`Arena`] can contain references to other items in
the same arena, but the crate feature `dropck_eyepatch` must be enabled
(which requires Rust nightly), as fixed-typed-arena must use the
[unstable feature of the same name][dropck_eyepatch].

[dropck_eyepatch]: https://github.com/rust-lang/rust/issues/34761

Alternatively, you may be able to use a [`ManuallyDropArena`] instead.

ManuallyDropArena
-----------------

This crate also provides [`ManuallyDropArena`], a type like [`Arena`] that
returns references of any lifetime, including `'static`. The advantage of
this type is that it can be used without being borrowed, but it comes with
the tradeoff that it will leak memory unless the unsafe [`drop`] method is
called.

[`Arena`]: https://docs.rs/fixed-typed-arena/0.2/fixed_typed_arena/struct.Arena.html
[`ManuallyDropArena`]: https://docs.rs/fixed-typed-arena/0.2/fixed_typed_arena/struct.ManuallyDropArena.html
[`drop`]: https://docs.rs/fixed-typed-arena/0.2/fixed_typed_arena/struct.ManuallyDropArena.html#method.drop
