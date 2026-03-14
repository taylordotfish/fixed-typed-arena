Changelog
=========

0.3.4
-----

* `manually_drop::ManuallyDropArena` is now `arena::ManuallyDropArena`. For
  compatibility, `manually_drop` still provides `ManuallyDropArena` via a
  re-export, but the module is now deprecated.
* Various internal code improvements.

0.3.3
-----

* The wrapper `Bool` and `Usize` types used by `TypedOptions` are now imported
  from [`integral_constant`]. This enables easier integration with other crates
  that use `integral_constant`.
* Documented option defaults.

[`integral_constant`]: https://docs.rs/integral_constant

0.3.2
-----

* Internal-only change: the [`add-syntax`] crate is now used to conditionally
  apply `unsafe` to a `Drop` impl when `dropck_eyepatch` is enabled.

[`add-syntax`]: https://docs.rs/add-syntax

0.3.1
-----

* Replaced the fundamental options struct with `TypedOptions`, which is
  parameterized with types instead of const generics. This enables more
  flexible metaprogramming. `Options` is now an alias of `TypedOptions`, and
  is still the recommended option interface for most users; `TypedOptions` is
  needed only in certain special circumstances.
* Fixed unsoundness in the implementation of `Sync` for `Iter`.
* Clarified safety requirements for unsafe arena methods.

0.3.0
-----

Note: this version contains a soundness issue and should not be used.

* Added support for iteration.
* Added a new option, `SupportsPositions`, which controls whether iterators
  can be converted to and from non-lifetime-bound `Position` objects.
* Added a new option, `Mutable`, which controls whether the arena can return
  mutable references.
* Arena types are now parameterized with an `Options` struct and trait.
  For compatibility and convenience, the arena types at the crate root are type
  aliases that still take the options as const generics. All options now have
  default values.

This version is backward-compatible with 0.2.3, with the following exceptions:

* The MSRV is now 1.59.
* The implementations of `Send` and `Sync` for `ManuallyDropArena` now require
  `T` to be `Sync`.

0.2.3
-----

* Implemented `Sync` for `ManuallyDropArena`.
* Improved internal architecture.

0.2.2
-----

* Added `ManuallyDropArena::manually_drop` as an alias of
  `ManuallyDropArena::drop`.

0.2.1
-----

Minor documentation updates.

0.2.0
-----

* Const generics are now used instead of `typenum`. Note that there is no
  longer a default chunk size, as defaults for const generics are not yet
  supported by Rust.
* Implemented `Send` for `Arena`.
* Increased MSRV to 1.51.

0.1.2
-----

* Fixed soundness issue when chunk size is 0.

0.1.1
-----

Note: this version contains a soundness issue and should not be used.

* `handle_alloc_error` is called when the global allocator fails, instead of
  simply panicking.

0.1.0
-----

Initial release.
