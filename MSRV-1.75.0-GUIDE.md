# MSRV 1.75.0 Guide

This guide is for users who want to use **opencc-jieba-rs** with Rust 1.75.x
or other older Cargo/toolchain setups.

Most users on modern Rust/Cargo with a fresh dependency resolution do NOT need
this guide.

---

## Background

Some older or locked dependency graphs can resolve `libflate <= 2.1.x`.
Those versions depend on the crate `core2`, which has been **yanked** from
crates.io.

- Modern fresh resolution usually selects newer versions such as
  `libflate >= 2.2` -> no special action needed
- Older toolchains, existing `Cargo.lock` files, or constrained dependency
  graphs may retain older versions -> manual fixes may be needed

---

## Step 1: Inspect dependency tree

Check if `core2` exists:

```bash
cargo tree -i core2
```

If nothing is shown, this guide does not apply to your current dependency graph.

---

## Step 2: Pin compatible versions (recommended)

Pin to known working versions:

```bash
cargo update -p include-flate --precise 0.3.0
cargo update -p libflate --precise 2.1.0
```

Then build with:

```bash
cargo build --locked
```

---

## Step 3: Patch yanked crate (if needed)

If Cargo fails due to `core2`, add:

```toml
[patch.crates-io]
core2 = { git = "https://github.com/bbqsrc/core2", rev = "545e84bcb0f235b12e21351e0c69767958efe2a7" }
```

---

## Notes

- This is only needed for older toolchains or dependency graphs that still
  resolve `core2`.
- Modern fresh dependency resolutions usually do not need any special setup.
- Applications (not libraries) should commit `Cargo.lock`.

---

## Recommendation

If possible, upgrade Rust:

```bash
rustup update
```

Using a modern Rust/Cargo toolchain usually provides a cleaner dependency graph
with no yanked crates.
