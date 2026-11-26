# Vendored Jieba performance review

Accepted incremental connector offsets in search mode and streaming tokens into OpenCC conversion. Kept Trie lookahead at 3 and the existing HMM emission callbacks.

## Method

The baseline was the working tree at `2fe72e3` as of 2026-11-26. The baseline executable and measurements were captured before any production changes. The ignored local novel stayed in place and was read from disk at runtime. Its UTF-8 file is 3,288,254 bytes.

All throughput runs used Rust 1.75.0 with LLVM 17.0.6 on `x86_64-pc-windows-msvc`, with the standalone benchmark package's release profile (opt-level 3, one codegen unit, fat LTO, panic abort). The host was a Windows machine with an Intel Core i5-13400 and 16 logical processors. The wrapper ran in a one-thread Rayon pool for serial dispatch and a four-thread pool for parallel dispatch. Short inputs still use the library's normal serial threshold in both pools.

The benchmark reuses the existing harness and allocator. Each row warms up the operation, calibrates a batch targeting at least 40 ms, then records seven samples. The timing includes creation and drop of the returned result. A separate warmed call counts allocation and reallocation requests and sums their requested sizes across threads. These bytes are cumulative requests per call, including the full requested size on reallocation. Dictionary and pool setup is outside the timed cases. Builds, snapshots, and timed processes ran sequentially.

The synthetic fixtures cover Traditional Chinese, mixed CJK and ASCII, repetitive OOV-heavy text, a long uninterrupted CJK block, connector-heavy identifiers, and a short sentence. The matrix covers `cut` and search with HMM enabled and disabled, full mode, the wrapper's segmentation, and the public `s2twp` conversion. Full mode has no HMM setting. The public conversion uses HMM internally.

## Baseline stage profile

The stage profile used a separate build of the distance-3 baseline. Temporary timers recorded each splitter step, the Trie walk with DAG edge insertion, DAG finalization, `calc`, inclusive `cut_route`, route emission with DAG clear, and HMM fallback. A separate replay traversed the same Trie matches with a counting callback to show the walk without edge writes. The table reports median shares of instrumented `cut()` wall time over five samples. Inclusive and nested measurements overlap. Per-block and per-fallback timers add overhead, especially on inputs with many small blocks. Use the release measurements for comparisons between variants.

| Fixture | HMM | Split | Trie replay | Trie + edges | DAG finalize | DP | Route inclusive | Route emission | HMM fallback |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Novel | Off | 8.9% | 23.6% | 30.2% | 2.8% | 10.2% | 75.2% | 15.5% | 0.0% |
| Novel | On | 8.3% | 22.1% | 27.5% | 2.5% | 9.4% | 80.2% | 26.4% | 12.2% |
| Mixed | On | 12.4% | 9.0% | 11.0% | 4.3% | 6.2% | 71.7% | 24.3% | 10.0% |
| OOV | On | 8.0% | 19.1% | 24.2% | <0.1% | 7.9% | 91.6% | 59.5% | 34.9% |
| Uninterrupted CJK | Off | 13.7% | 49.5% | 56.9% | <0.1% | 19.1% | 85.8% | 9.3% | 0.0% |

[Stage measurements for all fixtures and both HMM modes](conservative-performance-stages.csv)

## Trie lookahead

The disabled variant removes the speculative next-slot touch. The distance-2 and distance-4 variants change only the lookahead distance. All three preserve the same traversal and yielded byte-for-byte identical snapshots.

Disabling lookahead helped several synthetic core cases but regressed the real novel in paired reruns. Serial conversion medians were 195.35–200.93 ms with lookahead disabled and 182.42–192.87 ms at distance 3. HMM-on core `cut` medians were 60.73–70.25 ms with it disabled and 55.79–56.90 ms at distance 3.

Distance 2 improved the geometric mean of the 35 core cases by 4.0% in its first run and 0.8% in its second run, relative to the geometric midpoint of the first two baseline runs per case. Its second run had OOV wrapper regressions of up to 8% and an uninterrupted-CJK `cut` regression of about 11%, each relative to the slower baseline run. The distance-4 first pass regressed serial OOV conversion by more than 10% relative to both baseline runs. Distance 3 had the more consistent trade-off across the measured workloads.

## Connector offsets

Search mode now advances byte and Unicode positions once per part. It counts each part even when that part is skipped, preserving the offsets of later results and the ordering of connector parts, dictionary grams, and the original token. Ordinary connector fixtures showed small, inconsistent timing differences. For single HMM-joined identifiers with many connectors, the repeated measurements show the benefit of removing repeated prefix scans.

| Input size | Distance-3 baseline medians | Connector-only medians | Allocation requests and bytes |
|---:|---:|---:|---:|
| 453 bytes | 6.10–6.13 µs | 5.32–5.38 µs | 1 / 10,848 |
| 3,589 bytes | 101.57–108.83 µs | 46.08–46.79 µs | 1 / 86,112 |
| 28,677 bytes | 3.810–3.814 ms | 0.396–0.402 ms | 2 / 589,824 |

## HMM emission lengths

The isolated candidate attached character counts to HMM emission callbacks. It used Viterbi span indices for Han words, a count of one for singletons, and the existing count for text emitted by the non-Han splitter. The recurrence, floating-point operations, and tie-breaking stayed the same. Built-in and runtime-loaded model snapshots matched exactly.

The initial OOV `cut` result improved from 59.71 µs to 53.79 µs. Paired reruns ranged from 52.03 to 57.01 µs for the candidate and from 57.70 to 58.10 µs for the baseline. OOV search slowed from 69.44–70.56 µs to 72.02–72.30 µs in those reruns. There was no consistent broader benefit or allocation reduction, so the HMM callback change was rejected.

## Streaming conversion

The streaming-only candidate reduced the novel's serial conversion from 193.21 ms to 183.68 ms and its four-thread conversion from 71.47 ms to 60.80 ms. Serial allocation requests fell from 468,189 to 37 and cumulative requested bytes fell from 196,992,376 to 29,930,104. With four threads, calls fell from 563,436 to 95,284 and bytes from 204,433,042 to 37,370,770. The OOV fixture's serial median was 3.3% slower in the isolated run, while its requested bytes fell from 405,632 to 12,416.

The conversion callback reads dictionaries and writes to its existing output buffer. The thread-local Jieba scratch is mutably borrowed during emission, so the helper is crate-private and its callback contract excludes reentry into segmentation on the same thread. The serial loop finishes a chunk before starting the next one. Each parallel task uses its executing thread's scratch. Lookup order, delimiter handling, and the forward maximum-matching fallback continue to use the existing conversion routines.

## Combined release results

The baseline below was rebuilt with the final benchmark harness. Combined values span the medians of two process runs. Timing covers the full 3,288,254-byte novel for every row.

| Operation | Baseline ms | Combined ms | Baseline MiB/s | Combined MiB/s |
|---|---:|---:|---:|---:|
| Core cut, HMM off | 53.67 | 55.09–59.41 | 58.43 | 52.78–56.93 |
| Core cut, HMM on | 61.22 | 60.00–60.20 | 51.22 | 52.09–52.27 |
| Full mode | 61.62 | 58.06–68.05 | 50.89 | 46.08–54.01 |
| Search, HMM off | 60.62 | 59.71–64.04 | 51.73 | 48.97–52.52 |
| Search, HMM on | 66.06 | 64.72–66.09 | 47.47 | 47.45–48.46 |
| Serial wrapper cut, HMM on | 106.23 | 106.11–111.28 | 29.52 | 28.18–29.55 |
| Four-thread wrapper cut, HMM on | 46.34 | 44.85–45.08 | 67.68 | 69.56–69.92 |
| Serial conversion | 199.92 | 167.03–184.57 | 15.69 | 16.99–18.77 |
| Four-thread conversion | 70.47 | 60.47–62.21 | 44.50 | 50.41–51.86 |

Across the 14 conversion cases, the geometric mean of combined elapsed-time ratios was 13.0–14.4% lower than the rebuilt baseline. Both combined novel conversion runs reproduced the allocation counts and bytes of streaming alone. The 28,677-byte HMM-joined connector case took 0.376–0.411 ms in the combined runs. The ordinary segmentation rows vary across repetitions, including HMM-off novel `cut`, which was slower than the rebuilt baseline in both combined measurements and within the span of the earlier baseline runs.

[All release measurements, including separate candidates and paired repeats](conservative-performance-measurements.csv)

## Correctness

All six independent variants and the combined build matched the baseline byte for byte in complete serialized snapshots. The snapshots contain token text, order, scalar offsets, and UTF-8 byte offsets for accurate, full, and search modes, along with POS tags, both HMM settings where supported, a user dictionary, a runtime-loaded HMM, and serial and parallel wrapper outputs. Conversion output was compared for three configurations over the full matrix, with a punctuation-enabled custom-word case as well.

The regression tests assert exact search output for a custom mixed-width compound, every token in a long joined identifier, and offsets from a separately configured runtime HMM. The conversion regression uses ordered dictionaries, a Jieba dictionary, a phrase requiring forward maximum matching, a supplementary-plane character, punctuation, CRLF, and one- and four-thread pools with both HMM settings.

All Rust 1.75.0 validation commands passed.

```powershell
cargo +1.75.0 test --offline --locked -p opencc-jieba-rs
cargo +1.75.0 test --offline --locked -p opencc-jieba-rs --all-features
cargo +1.75.0 test --offline --locked -p opencc-jieba-rs --no-default-features
cargo +1.75.0 check --offline --locked --workspace --all-features --all-targets
```

## Reproduction

Run these commands from the repository root with the novel already available locally. Use the original production sources when rebuilding the baseline and keep a separate executable for each variant.

```powershell
cargo +1.75.0 build --offline --locked --release --manifest-path benchmarks/Cargo.toml --bin jieba_baseline
& benchmarks/target/release/jieba_baseline.exe --review
```

`--review-snapshot` emits complete results for comparison between executables. The output includes novel text and belongs in a local ignored directory. On Windows, `Start-Process -RedirectStandardOutput` captures the original bytes. `JIEBA_REVIEW_CASE` selects a timed fixture. This review has separate CSVs from the historical runs.

`--review-connector-scaling` measures search mode at 32, 256, and 2,048 connector groups with both HMM settings.

## Measurement limits

The relative results characterize this Windows host and its MSVC release build. The two initial baseline runs varied substantially on some novel cases, including HMM-off core `cut` at 53.72–63.91 ms. Small deltas should be read alongside that spread. A four-thread pool helps when the wrapper can distribute multiple ranges. Uninterrupted fixtures can give it only one work item. The allocation totals describe request traffic, and parallel totals can vary with scheduling and per-thread scratch reuse.

The public conversion configurations in the timed matrix select HMM internally. The private conversion route is exercised with HMM on and off in the regression tests. Full-mode segmentation has its own path and takes no HMM argument.

## Changed files

| Path from repository root | Change |
|---|---|
| `src/jieba_rs/mod.rs` | Incremental connector offsets and crate-private word emission. |
| `src/opencc.rs` | Streaming conversion and a serial/parallel regression for precedence, fallback, and delimiters. |
| `tests/jieba_offsets.rs` | Exact search offsets and ordering, a long connector stress case, and offsets from a custom HMM. |
| `benchmarks/jieba_baseline.rs` | Timed review, exact snapshot, and connector scaling modes. |
| `benchmarks/Cargo.lock` | Local library version refreshed to 0.9.1 so benchmark builds can use `--locked`. |
| `benchmarks/jieba/CONSERVATIVE-PERFORMANCE.md` | This report. |
| `benchmarks/jieba/conservative-performance-measurements.csv` | Release timings, throughput, and allocation measurements. |
| `benchmarks/jieba/conservative-performance-stages.csv` | Instrumented stage samples. |
