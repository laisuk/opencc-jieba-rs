# Private Jieba maintenance notes

## Origin and confidence

The upstream Rust project is [jieba-rs](https://github.com/messense/jieba-rs),
an implementation of [Jieba](https://github.com/fxsjy/jieba).
The original integration scaffold claimed **jieba-rs 0.11.0**. This was an
importer's statement, not a verified source identity: the exact upstream version
and commit are **unknown / unverified**. No source archive checksum or upstream
revision accompanies the bundle. The local import was committed as `21d67d4` (Vendor Jieba core and document local
adaptations); that local commit does not authenticate the upstream baseline.
The previous external dependency was pinned to 0.7.4 (see the commented manifest
entry and historical changelog); that pin does not identify this bundle.
Historical module documentation mentioned other upstream versions; those claims
must not be used as provenance evidence. No full upstream source comparison or
drop-in compatibility audit has been performed.

## Why a private module

The current integration keeps control of the shared Hans/Hant dictionary,
embedded data decoding, internal data structures, and dependency/MSRV surface
inside this crate. `src/lib.rs` declares `pub(crate) mod jieba_rs`; this is not
a separately supported replacement crate. These are observable design reasons
for the present arrangement; the original author's complete vendoring rationale
is not recorded. Preserve Rust **1.75.0** when maintaining it.

## Current adaptations and API surface

The following describes the checked-in working source, including adaptations
already present on the branch. Without an authenticated upstream baseline, it
is not an exhaustive attribution of which changes originated locally.

- Imports use `crate::jieba_rs`; the parent re-exports `Keyword`, while the
  tokenizer module, models, and extractor implementations remain private to the crate.
  `OpenCC::jieba` exposes an instance of the local tokenizer for advanced use, but
  external Jieba types are not interchangeable. Prefer the parent `OpenCC` methods.
- `Jieba::empty()` is empty; `new()` and `Default` load the parent's embedded
  dictionary. There is no separate upstream default dictionary or
  `load_default_dict()` method. `with_dict` and `load_dict` accept `BufRead`.
  The original scaffold's claim that `new()` is empty was incorrect.
- `cut`, `cut_all`, and `cut_for_search` return `Vec<Token>` carrying word and
  position information; parent wrappers expose their own results. Do not assume
  signatures or token offsets match any particular upstream release.
- Dictionary matching uses the local character trie (`trie.rs`) and sparse DAG,
  interned POS tags, cached log frequencies, and reusable thread-local scratch
  buffers with retention limits. Character-class scanning is implemented locally
  rather than through regular expressions in this module.
- HMM segmentation uses generated indexed emission tables and reusable Viterbi
  buffers. `HmmModel` supports a custom segmentation model. POS uses a separate
  lazily loaded compound HMM and longest-span fallback tag selection.
- TF-IDF and TextRank use local hash aliases, a minimal float ordering wrapper,
  and heap ranking with explicit word tie-breaking. TextRank stores its graph
  in flat adjacency arrays and filters candidates using the extractor config.
  These implementations require regression review when importing changes;
  equivalent results across upstream versions have not been established.

## Dictionary and model integration

`src/opencc.rs::DICT_HANS_HANT_ZSTD` embeds
`src/dictionary_lib/dicts/dict_hans_hant.txt.zst`. Both `Jieba::new()` and the
parent constructor decode it through `crate::zstd::decompress` and pass a cursor
to `Jieba::with_dict`. `load_dict` parses word/frequency/optional-tag entries;
parent user-dictionary APIs add their own validation and transactional handling.
OpenCC conversion dictionaries are a separate representation and should not be
confused with the tokenizer lexicon.

`data/idf.txt.zst` and `data/posseg.txt.zst` are embedded with `include_bytes!`
and decoded lazily using `OnceLock`. The root `build.rs` reads `hmm.model` and
generates `OUT_DIR/hmm_data.rs`, included by `hmm.rs`; its emission index is
restricted to the segmenter's configured Han range. Update generator and consumer
together. Do not silently replace any of these linguistic assets during a code
update. Their exact source revisions and transformation commands are unrecorded.

## Dependencies and features

- The external `jieba-rs` dependency is commented out.
- `ordered-float` is replaced here by `keywords/ordered_float.rs` (f64 ordering
  only); see its module header for scope and provenance limitations.
- Hash maps/sets use `std::collections` with the parent's vendored
  `crate::rustc_hash::FxBuildHasher`, rather than an external rustc-hash dependency.
- The local trie avoids an external trie implementation; HMM table generation
  is in `build.rs`, with no `jieba-macros` dependency. The precise upstream
  dependency versions/replacement history have not been verified.
- This module uses `bytecount`, but no regex imports or `include-flate` macros.
  The parent still depends on `regex` for classification; it no longer depends on
  `include-flate`. Check actual usage before changing dependencies.
- Embedded Zstd decoding uses the parent's vendored decoder. Native `zstd` is
  optional and tied to the parent's `dictionary-build` feature.
- TF-IDF and TextRank use the parent's default-enabled `keywords` feature. POS
  tagging remains available without it. The parent has no `default-dict`, `tfidf`,
  or `textrank` feature switches.

## Tests

Local `cfg(test)` tests cover the trie, sparse DAG, HMM, POS, TextRank, and float
comparison semantics. HMM expectations use ordinary assertions (a commented
`expect_test` import remains); POS tests run without the old `default-dict`
gate. TF-IDF tests in `keywords/tfidf_tests.rs` cover shared built-in data, custom
loads, filtering, and concurrent first use; these run with `keywords`. Parent integration tests cover segmentation, tags, keywords, user
dictionaries, conversion, and public API contracts. This is not evidence that
the complete upstream test suite or feature matrix has been retained or passed.
All existing tests must remain intact during maintenance.

## Importing future upstream changes

1. Obtain a specific upstream tag and commit, retain its archive checksum,
   license, model/dictionary provenance, and a clean source baseline. Resolve
   the current unidentified baseline if possible before attempting a three-way
   merge. Record the evidence here; do not infer a version from similar APIs.
2. Review individual fixes against the private integration. Preserve dictionary
   ownership, API visibility, token/offset contracts, ordering/tie-breaking,
   custom models, user-dictionary behavior, and parent conversion semantics.
   Review model or dictionary updates as separate behavior changes.
3. Audit dependencies, licenses, feature gates, build generation, and APIs for
   Rust 1.75.0. Avoid importing newer standard-library APIs without an MSRV-safe
   adaptation. Update these notes with the source revision and local patch list.
4. Run `cargo +1.75.0 check --locked -p opencc-jieba-rs` and
   `cargo +1.75.0 test --locked -p opencc-jieba-rs`, including doctests and the
   existing segmentation/POS/keyword/user-dictionary/conversion regressions.
   Also test `--features dictionary-build` when affected and workspace/C API
   consumers when interfaces change. Compare outputs against an identified
   upstream baseline before claiming compatibility.
5. Review the distributable archive (`cargo package --list`) and installed
   notices, not just the Git checkout, before publishing.

## Licenses, attribution, and unresolved release questions

Upstream jieba-rs is MIT licensed. The root `JIEBA-LICENSE` contains the notice
for **Copyright (c) 2018 - 2019 messense** and **Copyright (c) 2019 Paul Meng**.
Retain that copyright and permission notice in copies or substantial portions
of the software, including distributions of this adaptation. Keep attribution
for any subsequently imported code or data; the parent's MIT declaration alone
does not document all bundled components. OpenCC-derived assets have separate
notices in `THIRD_PARTY_NOTICES.md` and `dicts/LICENSE`.

Outstanding checks before release:

- Authenticate the claimed Jieba 0.11.0 baseline and the origins/licenses of the
  HMM, POS, IDF, and combined Hans/Hant lexicon assets. The bundled source license
  does not establish the lineage of each repackaged data file.
- Establish whether the local OrderedFloat code was independently implemented
  or copied/adapted, and from which revision; upstream ordered-float is MIT
  licensed, but no matching import record or dedicated license copy is present.
- Packaging now explicitly includes `JIEBA-LICENSE`, this note, `hmm.model`,
  and compressed IDF/POS data; `THIRD_PARTY_NOTICES.md` describes the vendored
  implementation. Package verification checks these assets and the build script.
  This corrects the earlier scaffold's packaging omissions without authenticating
  the unresolved upstream/data provenance above.
