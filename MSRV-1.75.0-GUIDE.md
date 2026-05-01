# MSRV 1.75.0 Guide

This guide is for users who want to use **opencc-jieba-rs** with **Rust 1.75.0**.

Most users on modern Rust (1.81+) do NOT need this guide.

---

## Background

Some older dependency chains (e.g. `libflate <= 2.1.x`) depend on the crate `core2`,
which has been **yanked** from crates.io.

- Modern Rust (1.81+) resolves newer versions (e.g. `libflate >= 2.2`) → ✅ no issue
- Rust 1.75 may still resolve older versions → ⚠️ may require manual fixes

---

## Step 1: Inspect dependency tree

Check if `core2` exists:

```bash
cargo tree -i core2
```

If nothing is shown, you are already fine.

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

- This is only needed for **older toolchains (Rust 1.75)**.
- Modern Rust users do NOT need to do anything.
- Applications (not libraries) should commit `Cargo.lock`.

---

## Recommendation

If possible, upgrade Rust:

```bash
rustup update
```

Using Rust 1.81+ provides a cleaner dependency graph with no yanked crates.
