# ruzstd decoder attribution

This directory contains a decoder-only adaptation of source code from **ruzstd 0.9.0**, part of the `zstd-rs` project by
Moritz Borcherding and contributors.

Upstream: https://github.com/KillingSpark/zstd-rs
Version used: 0.9.0
License: MIT (see `LICENSE-RUZSTD`)

OpenCC adaptation:

- removed encoders, `StreamingDecoder`, dictionary building and decoding-dictionary APIs;
- removed `no_std`, writer, sized-output, and unused upstream compatibility APIs;
- removed unused alternative ring-buffer implementations, retaining the active decoding algorithms;
- adjusted internal paths for nesting under `crate::zstd`;
- retained a specialized crate-private `decompress()` entry point for OpenCC dictionary loading;
- retained compatibility with Zstandard frames both with and without a declared frame content size (FCS).

See README.md for retained behavior, implementation details, and the dependency audit.