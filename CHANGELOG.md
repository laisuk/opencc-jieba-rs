# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.7.0.1] - 2025-09-15

### Changed

- Optimized zho_check() to scan only first 1000 bytes of input string.

### Fixed

- Fixed CLI tool opencc-jieba office pptx (temp file/directory creation error) and epub (Windows file/directory access denied error).

---

## [0.7.0] - 2025-08-23

### Added

- Add OpenOffice document and Epub conversion to CLI opencc-jieba.

### Changed

- Update STPhrases.txt
- Optimized token cut and conversion string heap allocations.
- Optimized general token cut with reduced string heap allocations
- Changed opencc-clip-jieba to use clap format as command arguments.

---

## [0.6.0] -2025-07-13

### Changed

- Improved performance with redesign `OpenCC-Jieba` **segmentation and conversion logic**.
- Improved **parallelism** implementation.

---

## [0.5.0] – 2025-06-18

### Added

- First official crates.io release of `opencc-jieba-rs`.
- Built with **Rust** and a **Jieba-style lexicon segmenter**, powered by **OpenCC lexicons** for Chinese text
  conversion.
- Support for:
    - Simplified ↔ Traditional (ST, TS)
    - Taiwan, Hong Kong, and Japanese variants
    - Phrase and character dictionaries
    - Punctuation conversion
- `Jieba` default to use **Large Dictionary** which supports both **Simplified and Traditional Chinese** text *
  *segmentation**.
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

