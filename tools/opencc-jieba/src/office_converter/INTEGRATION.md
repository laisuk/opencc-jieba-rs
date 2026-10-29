# Internal ZIP integration

The supplied staging source has been integrated as private `office_converter::zip`.
The core library and `opencc-tool-common` do not depend on this module.

`converter.rs` imports the private module. The CLI manifest owns `crc32fast`,
`flate2`, `indexmap`, and `memchr`; no standalone ZIP crate or external `zip`
dependency is used. The CLI integration test compiles the same source through a
private test harness, without exporting production APIs.

Stored and DEFLATE support are unconditional. Original ZIP feature conditions
were specialized to this configuration: enabled conditions removed, unsupported
conditions replaced by `cfg(any())`. CLI features cannot accidentally disable
DEFLATE or enable an unsupported codec. The Rust `flate2` backend replaces the
previous zlib-rs/Zopfli configuration; compressed bytes may differ, while the
compression format and decoded content remain compatible.

`ZipArchive::new` / `with_config` validate all indexed local headers before
exposing archive metadata. See [VENDOR_NOTES.md](VENDOR_NOTES.md) for the precise
security boundary, compatibility limits, source attribution, and verification.
The original advisory is preserved separately in [SECURITY_ADVISORY.md](SECURITY_ADVISORY.md).
