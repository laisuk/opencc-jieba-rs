# Vendoring notes: rustc-hash

## Port to opencc-jieba-rs

Copied from the local opencc-fmmseg/src/rustc_hash module on 2026-10-25.
The hashing implementation and regression tests are unchanged. This package
restores the upstream-style FxHashSet alias and std::collections::HashSet import
for dictionary key-length metadata. Both map and set aliases remain private
implementation details. The external rustc-hash dependency is removed.

The compatibility verification history below belongs to the source package
(opencc-fmmseg), not this port. See the port validation section for local checks.


## Upstream

- Project: `rustc-hash`
- Version: `2.1.3`
- Repository: <https://github.com/rust-lang/rustc-hash>
- Original license: MIT OR Apache-2.0
- Original upstream MSRV: Rust 1.77, verified from `rust-version = "1.77"`
  in both `Cargo.toml` and `Cargo.toml.orig` in the official 2.1.3 source
  distribution.
- Local compatibility target: Rust 1.75.0.

The comparison baseline is the official published source archive:
<https://static.crates.io/crates/rustc-hash/rustc-hash-2.1.3.crate>.

## Vendoring approach

The code is integrated as a private internal Rust module, declared by
`mod rustc_hash;` in `src/lib.rs`, rather than as a separate Cargo crate.
No separate `rustc-hash` Cargo dependency is required. Internal consumers use
`crate::rustc_hash`; existing public OpenCC APIs remain unchanged.

The upstream hashing algorithm and expected hash outputs must remain unchanged.
Preserve the upstream behavior for the corresponding architecture and stable
configuration; this does not imply identical outputs across architectures or
support for upstream optional nightly behavior.

## Local modifications relative to 2.1.3

- Upstream `src/lib.rs` becomes `src/rustc_hash/mod.rs`, with a local provenance,
  license, and compatibility header. The standalone crate examples in the module
  and `FxBuildHasher` documentation are removed.
- Crate-root imports in the retained tests use `super::`
  instead of `crate::`. The test `HashBytes` implementation uses the already
  imported `Hasher` name instead of `core::hash::Hasher`. Consumer imports use
  `crate::rustc_hash`.
- The bulk byte loop replaces `split_first_chunk::<16>()` with a
  `while bulk.len() >= 16` guard and `bulk.split_at(16)`, compatible with Rust
  1.75.0. The same 16-byte chunks, ordering, mixing, remainder, and suffix are
  retained. Redundant references/parentheses in chunk conversions are removed,
  and `suffix[0..8]` becomes `suffix[..8]`.
- Standalone `#![no_std]`, nightly feature attributes (`const_default`,
  `const_trait_impl`, `derive_const`, `hasher_prefixfree_extras`), and
  `#![allow(rustc::default_hash_types)]` are removed. This module uses `std`
  unconditionally; `std` feature guards on retained imports and aliases are
  removed. The redundant `extern crate std` and prelude `Default` import are
  removed.
- Optional `rand` integration (`extern crate rand`, `random_state`, and its
  re-exports) is omitted. Nightly-only `Hasher::write_length_prefix` and
  `Hasher::write_str` overrides are omitted. `FxHasher` and `FxBuildHasher`
  derive ordinary `Default` unconditionally instead of selecting between
  ordinary and nightly const derives.
- Unused seeded hashing APIs and their tests are omitted as detailed below.
  The source package removed the unused `FxHashSet` alias and `HashSet` import; this port restores them for dictionary metadata. Their
  associated dead-code and unused-import warning suppressions are removed.
  The redundant inherent `FxHasher::default` constructor is removed;
  `FxBuildHasher` uses the derived `Default` implementation, which still
  initializes the hash to zero.
- Other differences are whitespace and minor comment wording edits (the MCG
  article and the bit-reversal/rotation commentary). Hash constants, mixing
  operations, and upstream hash regression expectations are retained.

## Seeded hashing

Seeded hashing has no internal consumers and is intentionally omitted from this
private module. `seeded_state.rs`, its module declaration and re-exports,
`FxSeededState`, the seeded map/set aliases, and its three unit tests are removed.
`FxHasher::with_seed` and its `with_seed_actually_different` test are also removed.
The retained API consists of `FxHashMap`, `FxHashSet`, `FxBuildHasher`, and `FxHasher` with
zero-initialized default state. The unsigned, signed, and byte hash-output
regression tests remain unchanged.

## Compatibility verification

Before the seeded hashing cleanup, the user successfully ran `cargo test` with
`rustc 1.75.0` on Windows 7 x86 (32-bit). User-reported results across unit
tests, integration tests, and
documentation tests were:

- 253 tests passed.
- 0 failed.
- 20 ignored.
- The 32-bit hash regression tests passed.

This records the user's completed verification, not a rerun performed while
writing these notes. Ignored tests were not verified by that run. Other operating
systems, architectures, compiler versions, and optional upstream configurations
are not established as tested by these results. Package listing or package
verification on another host does not establish Windows 7 x86 compatibility.

After the seeded hashing cleanup, verification on 2026-10-25 with Rust 1.75.0
on Windows x86_64 (`x86_64-pc-windows-msvc`) used
`cargo test --offline -p opencc-fmmseg`: 184 unit/integration tests passed and
7 were ignored, including successful unsigned, signed, and byte hash regressions.
The initial documentation run encountered six temporary-directory write-access
errors. Rerunning `cargo test --offline -p opencc-fmmseg --doc` with `TEMP` and
`TMP` pointing to `target/rustdoc-temp` passed all 65 active documentation tests;
13 were ignored. Combined results were 249 passed, 0 failed, and 20 ignored.
`rustfmt --check --edition 2021 src/rustc_hash/mod.rs` and `git diff --check`
passed. `cargo package --list --allow-dirty --offline -p opencc-fmmseg` confirmed
that these notes and both licenses remain included and `seeded_state.rs` is
absent. This cleanup has not been reverified on Windows 7 x86.

## Licenses and packaging

`LICENSE-MIT` and `LICENSE-APACHE` were already present and verified byte for byte
against the official 2.1.3 source distribution. They retain their original text.
Their SHA-256 digests are:

- `LICENSE-MIT`: `30fefc3a7d6a0041541858293bcbea2dde4caa4c0a5802f996a7f7e8c0085652`
- `LICENSE-APACHE`: `95bd3988beee069fa2848f648dab43cc6e0b2add2ad6bcb17360caf749802bcc`

The root manifest explicitly includes this file and both upstream licenses in
the published package. Verify with `cargo package --list --allow-dirty` and,
when the environment permits, `cargo package --allow-dirty`. Do not publish as
part of this verification.

## Maintenance

For an upstream update, obtain the exact release source distribution and compare
it against both the current upstream baseline and these local files. Review
changes individually and selectively backport fixes while retaining Rust 1.75.0
compatibility, the private module paths, the trimmed API, and original licenses.
Update provenance and this modification inventory to reflect the actual source.
Do not automatically adopt upstream MSRV increases, optional dependencies,
nightly features, or algorithm/output changes.

Preserve hash-output stability for each supported architecture and retain the
regression expectations. Rerun the full suite on Windows 7 x86 with Rust 1.75.0,
including the 32-bit hash regression tests, after source backports; verify other
targets separately and label their results explicitly. Recheck license text
against the selected release and confirm all three documentation/license files
remain in the Cargo package listing.

## Port validation (2026-10-25)

On Windows x86_64 with rustc 1.75.0, cargo test --offline --workspace
--all-features passed (291 tests passed, 4 ignored), including the three
vendored hash regression tests and 93 documentation tests. The locked
no-default-features library check passed. Package listing confirmed the
module, these notes, and both license files are included. License SHA-256
digests match those recorded above. Formatting checks for the changed Rust
files and git diff --check passed; the workspace-wide formatting check found
existing differences in unrelated Zstandard and CLI files. Windows 7 x86
has not been verified for this port.

