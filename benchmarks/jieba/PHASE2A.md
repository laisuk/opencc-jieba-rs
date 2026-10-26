# Phase 2A — shared parsed default TF-IDF data

Baseline: `21d67d439f77e676e3bca33907ecdc5c24b1fbf3`.
This phase changes only `src/jieba_rs/keywords/tfidf.rs` in production code.
The existing keyword wrappers and configuration implementation were inspected
and require no changes: every `TfIdf::default()` now reuses the same parsed data.
Trie, DAG, HMM, POS, TextRank, dictionary transactions, public APIs, dependencies,
and linguistic assets are unchanged.

## Design and semantics

- `OnceLock<IdfData>` lazily decodes and parses the bundled IDF table once. The
  initialized map and median are immutable and shared safely between threads.
  Publication uses the standard library's initialization synchronization.
- `TfIdf` holds `Cow<'static, IdfData>`. Defaults borrow the static table, with
  no map clone, reference-count operation, or per-request map allocation.
  Configuration is still owned and independently mutable per extractor.
- `TfIdf::new` starts with owned, empty data, preserving custom-only dictionaries
  and the zero initial median. Loading custom data into a default extractor
  clones the table once on its first mutation; subsequent loads mutate its owned
  copy. No change leaks to the static table or other extractors.
- The parser, duplicate overwrites, duplicate-inclusive lower median for the
  current load, filtering, arithmetic, heap comparisons, and output ordering
  are unchanged. The existing partial-update behavior on I/O errors and panic
  on empty/all-invalid IDF loads are deliberately retained.
- Decoded built-in IDF text is dropped after parsing; the parsed table remains
  resident until process exit. This exchanges process-lifetime retained memory
  for lower repeated-request latency and allocation pressure; see memory data.
- This phase does not cache entire extractors or share mutable configuration.
  Stopword/config construction and per-call extraction allocations remain.

## Regression coverage

Seven tests were first run against the original production implementation;
`phase2a-baseline-regressions.txt` records that run and the captured fingerprints.
The final suite has eight new TF-IDF tests in `tfidf_tests.rs`:

- Compare every built-in map value and median by f64 bits with a separately
  decoded and BufRead-loaded custom table.
- Compare complete results and **64 frozen baseline fingerprints**, including
  words, tags, weight bits, and order, across Simplified/Traditional/mixed/OOV
  fixtures, both HMM settings, top-k 0/1/5/100, and POS filtering.
- Check custom-only loading, duplicate overwrites, even/odd lower medians,
  median replacement per load rather than across the merged map, tied ordering,
  invalid lines, infinities, NaN, and negative zero.
- Check mutation/config isolation, shared default table identity, and no second
  table copy for later custom loads.
- Preserve partial data updates with the old median on injected read failure,
  and preserve empty/all-invalid load panics.
- Race eight first-time default constructors and repeatedly extract on them;
  concurrently call weighted/unweighted/filtered public APIs using two OpenCC
  instances and verify exact outputs.

The original tests are unchanged. See the verification results below for commands.

## Reproduce the timing comparison

The existing `examples/jieba_baseline.rs` is **unchanged**, SHA-256
`476bfe5a67a1d26090050843e21fdd8d7576b3d1fd7eb9915b69132a2117a7e8`.
Use the same environment and method as `README.md` and `REPORT.md`: Rust 1.75.0,
locked default-feature release build, fat LTO, seven batches of at least 40 ms,
five fresh child processes, then one- and sixteen-worker Rayon runs. Timing
includes output destruction; allocation counters run separately. Do not run
builds/tests simultaneously with benchmarks.

```powershell
cargo +1.75.0 build --locked --release --example jieba_baseline --target-dir target/jieba-baseline
$env:RAYON_NUM_THREADS = '1'
& ./target/jieba-baseline/release/examples/jieba_baseline.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/phase2a-1-thread.csv
$env:RAYON_NUM_THREADS = '16'
& ./target/jieba-baseline/release/examples/jieba_baseline.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/phase2a-16-threads.csv
```

Restore your previous environment variable afterward. Baseline CSVs from Phase 1
are retained unchanged. New results use separate filenames.

## Timing results

The primary comparison below uses the original one-worker baseline CSV and the
first patched one-worker run. Times are microseconds, medians of seven batch
means, including result destruction. The corresponding sixteen-worker run also
completed all 144 rows. Fixtures and their limitations are unchanged.

| Operation/input | Baseline us | Patched us | Baseline → patched allocations | Baseline → patched requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Warm default TF-IDF construction + drop | 38,016.000 | 1.872 | 270,201 → 67 | 21,664,546 → 3,006 |
| Public TF-IDF, Simplified short | 39,288.600 | 4.473 | 270,227 → 93 | 21,667,970 → 6,430 |
| Public TF-IDF, Traditional short | 37,389.800 | 4.588 | 270,227 → 93 | 21,667,956 → 6,416 |
| Public TF-IDF, mixed short | 36,499.850 | 4.698 | 270,231 → 97 | 21,667,943 → 6,403 |
| Public TF-IDF, Simplified long | 37,484.450 | 232.036 | 270,227 → 93 | 21,667,970 → 6,430 |
| Public TF-IDF, Traditional long | 40,801.850 | 232.272 | 270,227 → 93 | 21,667,956 → 6,416 |
| Public TF-IDF, mixed long | 37,506.950 | 322.098 | 271,251 → 1,117 | 21,671,768 → 10,228 |

Public latency falls approximately 7,800–8,800-fold for these short fixtures and
116–176-fold for the repeated long fixtures. The cause is amortizing IDF parsing
and map allocation, not accelerating the segmentation/ranking algorithms.
Allocation counts and requested bytes fall identically in the sixteen-worker
TF-IDF rows, since this keyword path is serial. Its patched short public medians
are 4.148 / 5.098 / 4.983 us, and long medians 233.815 / 237.396 / 329.290 us
(Simplified / Traditional / mixed). No Rayon tuning was performed.

### First use

Fresh-process medians over five children, one-worker run:

| Operation | Baseline ms | Patched ms | Baseline min–max ms | Patched min–max ms |
| --- | ---: | ---: | ---: | ---: |
| First core default TF-IDF initialization | 57.530 | 54.014 | 51.394–59.277 | 53.657–58.021 |
| First public TF-IDF extraction | 85.989 | 75.349 | 81.879–97.522 | 73.142–81.829 |

The patch does **not** eliminate the first parse. The memory probe confirms
identical first-default cumulative allocation requests. Small cold-latency
changes should not be interpreted as a proven startup optimization: these are
fresh-process observations with warm OS caches, fixed prior-operation order,
and no affinity/frequency lock. First public TF-IDF can also initialize POS data.

### Reused core extractors and other regression checks

Reused core extraction is not the target optimization. The arithmetic, term
frequency iteration, filtering, and heap ranking remain the same; only table
ownership and access change. Allocations/requested bytes remain **exactly
unchanged** for every reused core TF-IDF row in both primary runs.

| Reused core input | Baseline us, 1 worker | Patched us, 1 worker | Change | Change in 16-worker process |
| --- | ---: | ---: | ---: | ---: |
| Simplified short | 2.183 | 2.121 | -2.82% | +9.65% |
| Traditional short | 2.335 | 2.288 | -2.03% | +1.77% |
| Mixed short | 2.627 | 2.619 | -0.32% | -5.57% |
| Simplified long | 229.432 | 242.024 | +5.49% | +10.71% |
| Traditional long | 227.192 | 234.600 | +3.26% | +0.77% |
| Mixed long | 312.851 | 316.647 | +1.21% | -0.36% |

These observed slowdowns are reported rather than hidden by the public-API
speedup. The sixteen-worker process still runs core TF-IDF serially. Unchanged
controls also moved: Simplified long core cut HMM off was +8.29%, HMM on -9.84%,
and TextRank +8.87% in that process. Thermal/scheduling/cache and LTO code-layout
effects are plausible confounders; the primary runs alone do not distinguish a
small reusable-extractor regression from run variation. A complete paired
baseline/patched one-worker rerun uses the preserved original baseline
executable to investigate this without altering the benchmark harness.

The back-to-back rerun completed with the same binaries and methodology:

| Reused core input | Paired baseline us | Paired patched us | Change |
| --- | ---: | ---: | ---: |
| Simplified short | 2.373 | 2.436 | +2.69% |
| Traditional short | 2.282 | 2.284 | +0.09% |
| Mixed short | 2.584 | 2.671 | +3.37% |
| Simplified long | 222.229 | 220.100 | -0.96% |
| Traditional long | 227.717 | 222.880 | -2.12% |
| Mixed long | 306.693 | 319.683 | +4.24% |

The large Simplified-long regression did not reproduce. Mixed long did show a
possible small regression in this paired run: its baseline batch means ranged
303.129–310.617 us and patched means 315.733–321.920 us. It was +1.21% and -0.36%
in the primary comparisons, so there is insufficient evidence to assign a
stable overhead to shared ownership, but **a small reused-core timing regression
is not ruled out**. No changes to other kernels were made to chase these results.
Allocation counts/bytes remained identical for all reused TF-IDF cases, including
top-k variants, in the paired comparison too.

Paired public results confirm the main gain: short calls were 38.130 / 38.597 /
39.039 ms before versus 4.238 / 4.274 / 4.642 us after, and long calls were
40.270 / 38.595 / 38.995 ms before versus 247.425 / 232.157 / 324.373 us after
(Simplified / Traditional / mixed). Warm default construction/drop was 40.610 ms
before versus 1.903 us after. All primary allocation reductions reproduced.

Raw paired data: `phase2a-paired-baseline-1-thread.csv` and
`phase2a-paired-1-thread.csv`. The preserved baseline binary's SHA-256 matches
Phase 1: `a60960a3bc333e3b486c9cc0a644f51a7729dfee2e57c7a8331b235eabd565be`.
Patched timing binary SHA-256:
`1588c2d075ffcf00917c3883454cdcdf43409a3ddd2452ce3a4b8d38c334bde7`.

To repeat the paired timing experiment, preserve the baseline executable from
a release build of the exported baseline tree (copy the unchanged timing
harness into that tree's examples directory). Then execute that binary followed
by the patched binary with `RAYON_NUM_THREADS=1`, redirecting each to a distinct
CSV. Use a separate Cargo target directory for the baseline source tree.

The known, deterministic memory/custom-mutation regressions are documented
below. No allocation reduction outside default TF-IDF setup is claimed.

## Memory probe

`examples/tfidf_memory_probe.rs` is a separate, dependency-free allocator probe,
not a timing harness. It isolates IDF/config setup without constructing a Jieba
tokenizer or an OpenCC conversion dictionary. It counts successful allocation
requests, cumulative requested bytes, live requested-byte deltas, and incremental
peak requested bytes, subtracting the old allocation size on successful realloc.
It primes stdout before measuring and snapshots counters before printing rows.
This measures Rust allocator layout requests, **not** RSS, allocator overhead,
OS commitment, fragmentation, or a production process-wide peak. The
`resident_after_dropping_defaults` row reports live bytes relative to the initial
snapshot; its other columns are placeholders.

The exact same probe is compiled against baseline sources exported with
`git archive 21d67d4` and against the patched sources. Separate target directories
avoid cross-tree Cargo artifact reuse. To reproduce the baseline probe:

```powershell
New-Item -ItemType Directory -Force target/phase2a-baseline-source | Out-Null
git archive --format=tar --output=target/phase2a-baseline-source.tar 21d67d4 Cargo.toml Cargo.lock build.rs rust-toolchain.toml src capi/opencc_jieba_capi tools/dict-generate tools/opencc-clip-jieba tools/opencc-jieba tools/opencc-tool-common
tar -xf target/phase2a-baseline-source.tar -C target/phase2a-baseline-source
New-Item -ItemType Directory -Force target/phase2a-baseline-source/examples | Out-Null
Copy-Item examples/tfidf_memory_probe.rs target/phase2a-baseline-source/examples/tfidf_memory_probe.rs
cargo +1.75.0 build --locked --release --example tfidf_memory_probe --manifest-path target/phase2a-baseline-source/Cargo.toml --target-dir target/phase2a-legacy-build
& ./target/phase2a-legacy-build/release/examples/tfidf_memory_probe.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/phase2a-baseline-memory.csv
```

For the patched probe:

```powershell
cargo +1.75.0 build --locked --release --example tfidf_memory_probe --target-dir target/jieba-baseline
& ./target/jieba-baseline/release/examples/tfidf_memory_probe.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/phase2a-memory.csv
```

The first custom load on a default extractor is intentionally measured
separately: copy-on-write moves that map-copy cost to the uncommon mutation path.

| Memory-probe operation | Baseline allocation calls / requested bytes | Patched allocation calls / requested bytes |
| --- | ---: | ---: |
| First default, including lazy initialization | 270,282 / 45,027,103 | 270,282 / 45,027,103 |
| Second default while first is alive | 270,201 / 21,664,546 | 67 / 3,006 |
| Sixteen warmed defaults, dropping each | 4,323,216 / 346,632,736 | 1,072 / 48,096 |
| First custom load into a default extractor | 3 / 63 | 270,136 / 19,500,547 |
| Second custom load into that extractor | 3 / 63 | 3 / 63 |

First-construction live requested bytes rise by 25,705,750 at baseline versus
19,504,793 after the patch because the decoded text is freed. The incremental
first-construction peak is essentially unchanged: 27,866,806 versus 27,862,497
bytes. The second overlapping extractor retains another 19,503,490 bytes at
baseline versus only 3,006 after the patch.

**Retained-memory regression:** after dropping all default extractors, the
baseline retains 6,202,260 bytes relative to probe startup (decoded IDF text and
default stopwords), whereas the patch retains 19,501,787 bytes (parsed table and
default stopwords). That is **13,299,527 more process-lifetime requested bytes**,
approximately 12.68 MiB. The static table does not grow with request count or
OpenCC instance count. In contrast, overlapping baseline extractors each retained
another approximately 19.5 MB table until dropped.

**Custom-mutation regression:** the first custom load on a default extractor
must now allocate an isolated approximately 19.5 MB table copy. The baseline
already owned its private map before this operation. Later loads do not clone
again; direct custom-only construction starts owned and avoids the shared-table
copy altogether. This is an explicit tradeoff, not a claim that all IDF loading
paths become faster. Data, configuration, and mutation isolation are preserved.

Raw probe results are `phase2a-baseline-memory.csv` and `phase2a-memory.csv`.
Probe source SHA-256:
`da017d6c88451fea9321d173054edfe5d7a9e92df1281e214e69652d5f638ee7`.
Baseline probe executable SHA-256:
`ac8ea9f459120f919a972b9305780d840d6f9f6d0f314da1b821b46244888243`.
Patched probe executable SHA-256:
`2a4cc020f412efa70fa13990d588d5cb40434e920d79572f330547ccf2c6585a`.

## Verification and changed files

- The seven initial behavioral tests passed against the baseline implementation
  before the optimization; the final eight tests retain those checks and add
  representation/mutation-isolation assertions. The concurrency test also
  shares one immutable extractor across threads.
- A fresh test process runs only the eight-way initialization race via
  `cargo +1.75.0 test --locked --all-features jieba_rs::keywords::tfidf::tests::default_initialization_and_repeated_extraction_are_thread_safe -- --exact`,
  ensuring other tests cannot pre-initialize the cache. See
  `phase2a-first-use-concurrency.txt`.
- Required full command: `cargo +1.75.0 test --locked --all-features`.
  **170 unit/integration tests and 93 doctests passed**, 3 doctests ignored,
  no failures or warnings. (A subprocess contract-test result is also printed
  inside the integration-test log; it is not counted twice.) Final output is
  `phase2a-all-features-tests.txt`.
- The release timing harness and both memory probes build with Rust 1.75.0.
  Four timing runs complete with 144 data rows each: patched 1/16 workers and
  the additional paired baseline/patched one-worker run.
- Production change: `src/jieba_rs/keywords/tfidf.rs` only. New regression tests:
  `src/jieba_rs/keywords/tfidf_tests.rs`. New memory probe:
  `examples/tfidf_memory_probe.rs`. This report and `phase2a-*` results/logs are
  isolated under `benchmarks/jieba/`.
- The original timing harness, Phase-1 results, existing tests, Cargo manifest,
  lockfile, build script, and all other production modules remain unchanged.
  Existing untracked workflow files are untouched. No commit was made.
- `rustfmt +1.75.0 --check` passes for the changed Rust files, and
  `git diff --check` passes. Primary and paired CSVs each have 144 complete rows
  with valid min/median/max ordering.

Patched production source SHA-256 (`tfidf.rs`):
`4321c78a444a3b616aefbfee10dcd67dd541615673648b2231b3e29476986562`.
The Phase-1 one-/sixteen-worker CSV hashes remain
`8a738139f926e7b9d4d0425d10c8afbc6f1a8772e17fb31138a883912e5d12bc`
and `8605494d06a9606bd5ef5ea3501e8f927e9d8bf775c49917f22ff295c49aae8b`.
