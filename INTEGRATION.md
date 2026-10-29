# Vendored Jieba integration

The integration is already part of this repository; no source-copying setup is needed.
The local import is commit `21d67d4`. Its exact upstream revision remains unverified;
the original scaffold's claimed jieba-rs 0.11.0 version is not authenticated.

- `src/lib.rs` keeps `jieba_rs` private to the crate; `OpenCC` is the high-level API.
- `OpenCC::jieba` exposes a local tokenizer instance. It is not type-compatible with
  the external `jieba_rs::Jieba`; low-level segmentation returns token records.
- `Jieba::new()` and `Default` load the embedded combined Hans/Hant dictionary;
  `Jieba::empty()` creates an empty tokenizer.
- The root build script generates HMM tables from `src/jieba_rs/hmm.model`.
- POS and IDF assets decode lazily through the private pure Rust Zstandard decoder.
- The default-enabled `keywords` feature gates TF-IDF and TextRank. POS tagging
  and segmentation remain available without it.
- Native `zstd` is optional and used for dictionary-pack compression through
  `dictionary-build`; runtime decoding needs no native Zstandard dependency.

See [maintenance and provenance notes](src/jieba_rs/VENDOR_NOTES.md),
[Jieba license](JIEBA-LICENSE), and [third-party notices](THIRD_PARTY_NOTICES.md).
The README lists the feature matrix, migration guidance, and release integrity checks.
