# Vendored ZIP 3.0.0

## Source and license

Source: user-supplied trimmed `zip2-3.0.0.zip`, from
[zip-rs/zip2](https://github.com/zip-rs/zip2), crates.io package `zip` **3.0.0**.
The installed registry copy of that exact version's manifest was inspected for
dependencies and Rust 1.75.0 support. MIT license, copyright © 2014 Mathijs van
de Nes, preserved verbatim in [zip/LICENSE](zip/LICENSE). Upstream package authors:
Mathijs van de Nes, Marli Frost, Ryan Levick, and Chris Hennick.

Vendored exclusively inside the CLI's private `office_converter::zip` module to
retain the ZIP 3 API and Rust 1.75.0 compatibility while backporting validation.
This is not a separate Cargo crate. The core library and `opencc-tool-common`
were not changed.

## Integration and trimming

Inspection found the staging tree was not connected to the CLI, needed direct
support dependencies, and still used upstream feature gates. Its mechanically
rewritten internal paths compiled after wiring and dependency migration; no
additional unresolved module declarations or algorithm damage were found during
integration. A working integration baseline passed the existing converter tests
before the security changes.

Preserved: `ZipArchive`, `ZipFile`, `ZipWriter`, `CompressionMethod`, `FileOptions`,
`ExtendedFileOptions`, `raw_copy_file`, ordinary `Read`/`Write` streaming,
Stored/DEFLATE, ZIP64, CRC checking on decoded reads, error propagation, and
Office/EPUB selection/font/conversion semantics. EPUB mimetype remains first and
Stored. Path filtering still omits unsafe entries; malformed headers fail the
whole conversion before any output replacement.

Already removed in supplied staging: upstream examples, benches, fuzz corpora,
large fixtures, most upstream tests, `read/stream.rs` and `unstable::stream`.
Additionally removed here: `aes.rs`, `aes_ctr.rs`, `read/lzma.rs`, the remaining
`read_zipfile_from_stream` function and its `ZipFileData::from_local_block` helper,
and `shallow_copy_file` / `shallow_copy_file_from_path` (which intentionally wrote
CDFH aliases with mismatched LFH names). Unused root re-exports and standalone
crate feature instructions were removed. The MIT license remains unchanged.

Optional upstream codec/time/fuzz branches remaining in shared files are disabled
with constant `cfg(any())`; Stored and flate2 DEFLATE branches are unconditional.
AES, Bzip2, Deflate64, Zstandard, LZMA, XZ, Zopfli and date-library adapters cannot
be enabled through parent CLI features. Upstream ZipCrypto supporting internals
and unused random-access/writer helpers remain to avoid a broader refactor; they
are private and covered by a module-local `allow(dead_code)`. No lint suppression
was added to the core library.

CLI direct dependencies added: `crc32fast = "1.4"`, `memchr = "2.7"`,
`indexmap = "=2.7.1"`, `flate2 = "=1.1.1"` with `rust_backend` and no default
features. The last two are pinned to compatible versions. The external `zip`
dependency was removed. Lockfile cleanup also removes ZIP-only Zopfli, zlib-rs,
libz-rs-sys, arbitrary/derive_arbitrary, bumpalo, and simd-adler32. The Rust
DEFLATE backend uses miniz_oxide/adler2; compression byte sequences may differ.

## Advisory investigation

Identifiers: **CVE-2026-115905**, **GHSA-m665-v7pm-7grj**.
The original supplied Dependabot text is [SECURITY_ADVISORY.md](SECURITY_ADVISORY.md);
its exploit demonstration was not executed.

ZIP 3's original `find_data_start` parsed the LFH lengths but skipped its name.
`ZipArchive` exposed CDFH metadata; its UnicodePath parser also replaced the raw
name, so comparing only parsed names would miss byte-level cloaking.
`raw_copy_file` does **not** copy whole input headers: it writes fresh headers
from the selected CDFH/effective name and copies compressed payload bytes. Thus,
the simple filename mismatch was canonicalized rather than copied verbatim, but
the malformed input was accepted. Removing streaming extraction alone did not
fix that parser differential. The retained shallow-copy APIs could independently
create such mismatches and have been removed.

Verified reference: upstream commit
[8068ab480ccf8daa7411424e365fcdbe6d09443e](https://github.com/zip-rs/zip2/commit/8068ab480ccf8daa7411424e365fcdbe6d09443e),
linked from the advisory. Inspection confirms upstream added opt-in consistency
validation for both readers, including raw names, UnicodePath identity, metadata,
and overlap checks. This is a focused implementation for the retained ZIP 3
configuration, not a verbatim backport or a claim of equivalence with ZIP 9.0.5.

## Exact security changes

`zip/validation.rs` runs unconditionally in `ZipArchive::get_metadata`, after the
central directory is collected and before `SharedBuilder::build` can hide duplicate
names. Both `new` and `with_config` use this boundary. It:

- Re-reads original CDFH and LFH filename bytes and requires exact equality.
- Requires matching UnicodePath fields, including version/CRC/content bytes, and
  rejects duplicate/short fields or truncated extra-field framing.
- Requires matching compression methods and encryption/data-descriptor/UTF-8 flags.
- Compares CRC and sizes, including ZIP64 local sizes. Data-descriptor entries may
  use zero local placeholders; populated local values must match the directory.
- Uses checked offset arithmetic, rejects payload/header spans crossing the central
  directory, duplicate effective names, and overlapping indexed entry spans.
- Caches validated data offsets for subsequent reads/raw-copy.

Failures return `ZipError` / `io::Error`, including descriptive header mismatch
errors. Ordinary malformed inputs do not reach output replacement. The shared
portable path filter now also rejects drive-relative names (`C:relative`),
alternate-data-stream names (`file:stream`), and NUL bytes, alongside absolute,
UNC, and parent-traversal names with either separator. The converter retains its
existing skip policy for unsafe paths. It does not extract input entries to disk.
Rebuilt output is reopened through the same mandatory validator before replacement.

## Regression coverage

`zip/tests.rs`: Stored/DEFLATE/ZIP64 and raw-copy round trips; both directions of
LFH/CDFH name mismatch (including `by_index_raw` and an empty failed-copy output);
every truncated prefix of a small archive; malformed extra-field framing;
UnicodePath override and original-name disagreement; local method/flag/CRC/size
mismatches; ZIP64 size mismatches; central-directory and indexed-entry overlaps;
duplicate names; valid data-descriptor placeholders.

`security_tests.rs` extends private converter tests: DOCX/XLSX/PPTX/ODT/ODS/ODP/EPUB
conversion with raw binary assets; reopening and fully reading rebuilt entries;
EPUB ordering/compression; portable unsafe path omission; target and non-target
header mismatches preserving existing/absent destinations with temp cleanup.
Existing output lifecycle, same-file, font/XLSX/PPTX, and Windows failure tests
remain in place. The CLI integration harness uses the same private ZIP source.
All payloads are inert; no shell payload is run.

## Verification (2026-10-29, Windows x86_64 MSVC)

- `cargo +1.75.0 check -p opencc-jieba`: PASS.
- `cargo +1.75.0 test -p opencc-jieba`: PASS, 38 binary tests and 9 integration
  harness tests (8 ZIP tests run in both targets).
- `cargo clippy -p opencc-jieba --all-targets -- -D warnings`: PASS with the
  repository's active Rust/Clippy 1.75.0 toolchain.
- `cargo test --workspace`: PASS on Rust 1.75.0, including workspace unit,
  integration, CLI lifecycle, C API and documentation tests; existing ignored
  tests remained ignored.
- `cargo tree -p opencc-jieba`: no external `zip` package; `Cargo.lock` has no ZIP
  package entry. The private source is also used by tests, with no dev ZIP dependency.

An additional `cargo +stable clippy -p opencc-jieba --all-targets -- -D warnings`
run failed on existing core-library lints (`empty_line_after_doc_comments`,
`needless_late_init`, `unnecessary_map_or`, `manual_is_multiple_of`). Those unrelated
files were left untouched. The required active-toolchain Clippy command passes.

## Limitations and maintenance

This is scoped mitigation of the named dual-header issue in this converter, not
a general ZIP security audit or a blanket vulnerability-resolution statement.
Strict validation deliberately rejects ambiguous archives formerly accepted,
including duplicate effective names, overlapping entries, and UnicodePath fields
present in only one header. Synthetic package tests cover all supported formats;
no large real-world Office/EPUB fixture corpus or cross-platform CI was run here.

The parser does not reconcile every possible parser interpretation, unindexed
LFHs, trailing records, timestamp/permission parity, or data-descriptor trailer
contents. Raw-copy preserves compressed payloads without decompressing them to
verify CRCs; the normal decoded-read checks are retained. No archive resource
quotas or decompression-bomb protection were added. Windows destination replacement
retains its prior non-atomic guarantee. Readers must supply stable input bytes
through the operation; concurrent mutation of the underlying file is not hardened.

Vendoring removes automated `zip` dependency alerts, **not** maintenance duties.
Maintainers must track upstream/RustSec advisories, assess fixes against these
retained paths, keep the license and this delta current, run the MSRV and workspace
checks on dependency updates, and extend adversarial coverage as needed. Do not
close an advisory solely because the external dependency vanished from Cargo.lock.
