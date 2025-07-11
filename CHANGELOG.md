# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.6.0] -2025-07-12

### CHANGED
- Improved performance by redesign `OpenCC-Jieba` **segmentation and conversion logic**.
- Improved **parallelism** design.

---

## [0.5.0] – 2025-06-18

### Added
- First official crates.io release of `opencc-jieba-rs`.
- Built with **Rust** and a **Jieba-style lexicon segmenter**, powered by **OpenCC lexicons** for Chinese text conversion.
- Support for:
    - Simplified ↔ Traditional (ST, TS)
    - Taiwan, Hong Kong, and Japanese variants
    - Phrase and character dictionaries
    - Punctuation conversion
- `Jieba` default to use **Large Dictionary** which supports both **Simplified and Traditional Chinese** text **segmentation**.
- `Dictionary` structure to preload dictionaries for Jieba.
- Built-in `Zstd-compressed JSON dictionary` loading.
- Methods to `serialize/deserialize` dictionaries (JSON and compressed).
- **Thread-parallel support** via `Rayon` for large text input.
- Utility for UTF-8 script detection (`zho_check`).
- **CLI** and **FFI** compatibility planned via workspace.

### Changed
- N/A

### Removed
- N/A

---

## [Unreleased]

