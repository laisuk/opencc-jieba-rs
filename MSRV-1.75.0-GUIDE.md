# MSRV 1.75.0 Guide

This guide is for users who want to use **opencc-jieba-rs** with Rust
1.75.x or other older Cargo/toolchain setups.

Most users on modern Rust/Cargo with a fresh dependency resolution do
NOT need this guide.

------------------------------------------------------------------------

## Background

Some older or locked dependency graphs can resolve `libflate <= 2.1.x`.
Those versions depend on the crate `core2`, which has been **yanked**
from crates.io.

- Modern fresh resolution usually selects newer versions such as
  `libflate >= 2.2` -\> no special action needed
- Older toolchains, existing `Cargo.lock` files, or constrained
  dependency graphs may retain older versions -\> manual fixes may be
  needed

------------------------------------------------------------------------

## Step 1: Inspect dependency tree

Check the dependency versions that can affect Rust 1.75.0 compatibility:

``` bash
cargo tree -i core2
cargo tree -i rayon-core
```

`core2` only matters when an older `libflate` dependency graph resolves
it.

For Rayon, this project pins `rayon = 1.10.0`. The known working
transitive `rayon-core` version for the Rust 1.75.0 build is `1.12.1`.

------------------------------------------------------------------------

## Step 2: Pin compatible versions (recommended)

For an older or constrained dependency graph, pin the known working
versions as needed:

``` bash
cargo update -p include-flate --precise 0.3.0
cargo update -p libflate --precise 2.1.0
cargo update -p rayon-core --precise 1.12.1
```

The `rayon-core` command prevents a fresh dependency resolution from
selecting a newer `rayon-core` release if its MSRV is no longer
compatible with Rust 1.75.0.

Then build with:

``` bash
cargo build --locked
```

------------------------------------------------------------------------

## Step 3: Patch yanked crate (if needed)

If Cargo fails due to `core2`, add:

``` toml
[patch.crates-io]
core2 = { git = "https://github.com/bbqsrc/core2", rev = "545e84bcb0f235b12e21351e0c69767958efe2a7" }
```

------------------------------------------------------------------------

## Notes

- The `core2` workaround is only needed for older toolchains or
  dependency graphs that still resolve `core2`.
- `rayon = 1.10.0` is pinned directly by this project;
  `rayon-core = 1.12.1` is the known working transitive version for
  the Rust 1.75.0 build.
- The project keeps an optional commented `rayon-core = "=1.12.1"`
  direct pin in `Cargo.toml`. It only needs to be enabled if Cargo can
  no longer resolve a Rust-1.75-compatible `rayon-core` transitively.
- Modern fresh dependency resolutions usually do not need the `core2`
  workaround.
- Applications (not libraries) should commit `Cargo.lock`.

------------------------------------------------------------------------

## Recommendation

If possible, upgrade Rust:

``` bash
rustup update
```

Using a modern Rust/Cargo toolchain usually provides a cleaner
dependency graph with no yanked crates.

------------------------------------------------------------------------

## Notes

This MSRV policy applies specifically to **opencc-jieba-rs** as a
published **library**.

Maintaining Rust **1.75.0** compatibility is an intentional project goal
and is considered part of the library's public compatibility contract.
It is **not** merely a recommendation for users of older Rust
toolchains. All releases in the current release line are expected to
build successfully with Rust **1.75.0**, and the project includes a
dedicated MSRV CI workflow to verify this guarantee.

As a result:

- Dependency updates are evaluated not only for new features, bug
  fixes, and performance improvements, but also for their impact on
  the library's effective MSRV.
- Changes that would raise the library's effective MSRV are carefully
  considered alongside their compatibility impact. When appropriate,
  such changes may be scheduled for a future release that
  intentionally raises the project's MSRV.
- Preserving MSRV compatibility helps downstream libraries and
  applications adopt **opencc-jieba-rs** without being forced to
  upgrade their Rust toolchain.

This policy applies **only to the library**. Companion applications,
examples, benchmarks, and GUI front-ends may intentionally target newer
Rust editions or compiler versions in order to take advantage of modern
language and standard library features. Their platform requirements are
independent of this library's compatibility policy.

In short:

> **Libraries prioritize long-term compatibility for downstream users.
> Applications are free to adopt newer platform features when
> appropriate.**
