# ruzstd decoder attribution

This directory contains a decoder-only adaptation of source code from **ruzstd 0.9.0**, part of the `zstd-rs` project by
Moritz Borcherding and contributors.

Upstream: https://github.com/KillingSpark/zstd-rs
Original baseline: 0.9.0
License: MIT (see `LICENSE-RUZSTD`)

Selected decoder changes were backported
from [ruzstd v0.9.1](https://github.com/KillingSpark/zstd-rs/releases/tag/v0.9.1)
and these subsequent upstream commits:

- [91a3a6eb](https://github.com/KillingSpark/zstd-rs/commit/91a3a6eb) applies the decoded-size limit to literals,
  including blocks without sequences.
- [96b83ce9](https://github.com/KillingSpark/zstd-rs/commit/96b83ce9) rejects reserved sequence compression mode bits.
- [d7f37b8c](https://github.com/KillingSpark/zstd-rs/commit/d7f37b8c) bounds compressed bodies before scratch-buffer
  growth.

OpenCC adaptation:

- removed encoders, `StreamingDecoder`, dictionary building and decoding-dictionary APIs;
- removed `no_std`, writer, sized-output, and unused upstream compatibility APIs;
- removed unused alternative ring-buffer implementations, retaining the active decoding algorithms;
- adjusted internal paths for nesting under `crate::zstd`;
- retained a specialized crate-private `decompress()` entry point for OpenCC dictionary loading;
- retained compatibility with Zstandard frames both with and without a declared frame content size (FCS).

See README.md for retained behavior, implementation details, and the dependency audit.