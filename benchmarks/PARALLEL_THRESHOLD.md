# Parallel threshold benchmark — 2026-10-31

## Recommendation

**Keep `PARALLEL_THRESHOLD = 5000` bytes and the existing single-worker guard.** This is the simplest supported shared threshold, giving priority to `jieba_cut_and_join` as requested. No further production change is needed.

On the 16-worker pool, 5,000 has the lowest measured excerpt score for joined segmentation with HMM both off and on. Forced parallel joined segmentation first wins across all three excerpts at approximately **5 KB** in both modes; at 4 KB it loses all three excerpts with HMM off and wins only two with HMM on. With four workers, both join modes consistently win by 4 KB, although the HMM-off gain there is only about 1.6%; 5 KB gives a clearer benefit. Treat the crossover as an approximate **4–6 KB region**, not an invariant byte count. The initial pass placed HMM-off crossover at 6 KB, reinforcing the need for a margin around a single observed result.

This recommendation is a workload compromise, **not a universal optimum**. If every measured operation and size receives equal weight, 3,000 bytes wins on 16 workers and 2,000 on four workers. Conversion-oriented applications could reasonably choose those lower values. Cut-all benefits later. The measurements do not justify additional per-operation thresholds, adaptive scheduling, runtime configuration, or a worker-count heuristic for this task.

## Candidate comparison

Scores below are geometric means of candidate median / forced-sequential median, over all 14 excerpt sizes and three positions. Lower is faster. Full-novel results are excluded because every candidate selects the same parallel path. Each operation, size, and excerpt has equal weight; these are explicit synthetic scores, not an inferred production request distribution. The summed medians column represents one call per excerpt/operation case (504 calls), not a separately timed batch.

| Threshold bytes | 16-worker all-op score | 16-worker join HMM off | 16-worker join HMM on | 16-worker summed medians ms | Four-worker all-op score |
|---:|---:|---:|---:|---:|---:|
| 1000 | 0.684 | 0.793 | 0.703 | 83.027 | 0.607 |
| 2000 | 0.661 | 0.760 | 0.682 | 82.448 | 0.600 |
| 3000 | 0.649 | 0.726 | 0.658 | 82.511 | 0.605 |
| 4000 | 0.655 | 0.712 | 0.652 | 82.955 | 0.617 |
| **5000** | **0.660** | **0.696** | **0.646** | **83.487** | **0.632** |
| 6000 | 0.672 | 0.701 | 0.655 | 84.728 | 0.654 |
| 7000 | 0.691 | 0.703 | 0.668 | 86.737 | 0.680 |
| 8000 | 0.717 | 0.723 | 0.690 | 89.629 | 0.712 |
| 9000 | 0.746 | 0.750 | 0.722 | 93.442 | 0.750 |

At 16 workers, 5,000 improves the joined-segmentation score over 1,000 by about 12.2% with HMM off and 8.1% with HMM on. Its all-operation score is approximately 1.7% worse than 3,000. At four workers, the join score at 5,000 is within about 0.2% of the best HMM-off candidate (4,000), and within about 1.7% of the best HMM-on candidate (3,000). Differences of this size should not be treated as statistically proven rankings from 12 observations.

## Crossover and throughput

The following is the first tested size where parallel has a lower median on **all three excerpts and every larger tested size**. This is observed consistency, not a statistical significance test. Small wins near parity are less convincing than the larger gains at the next size.

| Operation | 16 workers | Four workers |
|---|---:|---:|
| `jieba_cut_and_join`, HMM off | 5 KB | 4 KB (near parity); clear at 5 KB |
| `jieba_cut_and_join`, HMM on | 5 KB | 4 KB |
| `jieba_cut`, HMM off/on | 6 KB | 3 KB |
| Search cut, HMM off/on | 6 KB | 4 KB off / 2 KB on |
| Cut-all | 9 KB | 3 KB |
| POS tag, HMM off/on | 5 KB off / 2 KB on | 2 KB off / 1 KB on |
| `s2t` | 3 KB | 2 KB |
| `s2twp` | 2 KB | 1 KB |
| `t2s` on Simplified input | 2 KB | 2 KB |

The detailed tables provide median microseconds and input MiB/s at every requested size, plus parallel-win counts. `threshold-summary.csv` provides the observed P10–P90 and MAD for each individual excerpt/candidate. Do not interpret a single averaged excerpt as the entire corpus distribution.

For the complete novel, the following uses the actual 5,000-byte candidate. Sequential times use the forced serial branch in the same pool. Throughput and spread below describe the parallel candidate.

| Workers | Operation | Sequential median ms | Parallel median ms | Parallel P10–P90 ms | Parallel MiB/s |
|---:|---|---:|---:|---:|---:|
| 16 | Join HMM off | 65.644 | 20.082 | 19.675–22.522 | 156.16 |
| 16 | Join HMM on | 76.546 | 21.298 | 20.389–26.696 | 147.24 |
| 16 | `s2t` | 102.442 | 20.109 | 19.814–26.558 | 155.95 |
| 16 | `s2twp` | 206.030 | 38.935 | 38.306–39.695 | 80.54 |
| 4 | Join HMM off | 66.525 | 31.076 | 30.058–33.267 | 100.91 |
| 4 | Join HMM on | 74.766 | 33.704 | 32.215–34.757 | 93.04 |
| 4 | `s2t` | 105.350 | 37.417 | 35.574–40.244 | 83.81 |
| 4 | `s2twp` | 204.981 | 72.313 | 70.017–73.952 | 43.37 |

On the novel, all nine candidates execute identical branches. Their measured differences are timing variation, **not evidence that one threshold makes full-novel processing intrinsically faster**. Joined segmentation at the recommended threshold is about 3.27× faster than sequential with HMM off and 3.59× with HMM on at 16 workers. The HMM-on parallel P10–P90 spread is about 6.31 ms, illustrating why a single timing would be misleading.

## Scheduling observations and limits

- More workers are not automatically better for short requests. At the 500-byte excerpts, forced parallel join is roughly 5.65×/4.66× the sequential time (HMM off/on) on 16 workers, versus 2.61×/2.19× on four. Worker wakeups, scheduling, per-range intermediate buffers, and ordered collection compete with a very short serial operation. This experiment measures their combined cost; it does not isolate each component.
- The 16-worker pool improves full-novel throughput, but moves the short-input crossover later than four workers. Logical CPU count alone cannot predict the right threshold, especially on a heterogeneous CPU with shared execution resources. Four workers is a sensitivity check; it is not a recommendation to change global Rayon configuration.
- HMM changes work per chunk, POS tagging adds work, and conversion performs dictionary lookups and sometimes multiple passes. Cut-all and token-vector APIs have different allocation/merge costs from join. One byte threshold cannot optimize all of them simultaneously.
- Natural novel text contains many small delimiter-separated ranges. Byte length is a cheap proxy for work; a long punctuation-free string with only one range may not gain parallelism at all. Results do not generalize to every Chinese domain, dictionary customization, CPU, concurrent service load, or Rayon nesting pattern.
- No statistical claim is made for small candidate-score differences. The two excerpt passes place the useful join crossover in the same broad region, but show some run-to-run movement. A production workload distribution or more machines would be needed to defend a universal optimum.

## Validation

Final artifacts contain **136,224 measured intervals**: 2 pool sizes × 43 input cases (14 sizes × 3 excerpts + novel) × 12 operations × 11 dispatch settings × 12 iterations. Every case also has three warmup calls per dispatch setting and full-output equivalence assertions. All benchmark assertions passed. Matrix validation confirmed 12 observations per case, all requested settings, and actual lengths in the target-to-target+2 range.

`cargo test --offline --locked -p opencc-jieba-rs` passed: 174 unit/integration tests and 93 doctests; the local-novel test and three illustrative doctests were ignored as configured. Then `cargo test --release --offline --locked -p opencc-jieba-rs --test jieba_join novel_join_matches_flattened_tokens -- --ignored` passed explicitly, checking the complete novel against flattened segmentation with HMM off/on and five delimiters. `git diff --check` passed. The only tracked production-source diff remains the user's pre-existing `src/opencc.rs` edit.

## Scope and implementation

The starting working tree already changed `PARALLEL_THRESHOLD` from 1,000 to **5,000 UTF-8 bytes**, corrected its comment from characters to bytes, and introduced `use_parallel(input_len)` with a `rayon::current_num_threads() > 1` guard. These pre-existing edits were preserved. Benchmarking did not edit production Rust sources, public APIs, dependencies, dictionaries, or algorithms. Nothing was committed.

Six dispatch sites in `src/opencc.rs` use the shared predicate:

| Dispatch site | Affected operations |
|---|---|
| `phrases_cut_convert_cow` | `s2t`, `t2s`, `s2tw`, `tw2s`, `s2twp`, `tw2sp`, `s2hk`, `s2hkp`, `hk2s`, `hk2sp`, `t2tw`, `t2twp`, `tw2t`, `tw2tp`, `t2hk`, `t2hkp`, `hk2t`, `hk2tp`, `t2jp`, `jp2t` (20 public conversion helpers). Config-based dispatch and C/CLI wrappers inherit this behavior. Multi-round conversions reevaluate the threshold for each intermediate string. |
| `jieba_cut` | Standard segmentation, HMM off/on |
| `jieba_cut_for_search` | Search segmentation, HMM off/on |
| `jieba_cut_all` | Full segmentation, no HMM parameter |
| `jieba_tag` | POS tagging, HMM off/on |
| `jieba_cut_and_join` | Joined segmentation, HMM off/on |

All split into punctuation/whitespace-delimited ranges before dispatch. Rayon processes ranges with ordered indexed collection; the threshold controls whether this path runs, not its chunk size or worker count. Regex splitting, collection, and concatenation remain part of the operation. Keyword extraction has no independent threshold check; this experiment does not benchmark keyword scoring, compatibility normalization, or detofu.

Measured operations: `jieba_cut_and_join`, `jieba_cut`, `jieba_cut_for_search`, and `jieba_tag`, each with HMM disabled and enabled; `jieba_cut_all`; and `s2t`, `s2twp`, `t2s`. Conversion APIs always enable HMM internally and expose no HMM toggle. Punctuation conversion is disabled; join delimiter is one ASCII space. `s2twp` exercises multiple conversion rounds. `t2s` intentionally receives the identical Simplified source, so it represents a largely unchanged conversion workload, not a representative Traditional-to-Simplified corpus. Not every conversion dictionary configuration was individually timed.

## Environment and method

- Windows 11 Pro, version 10.0.26200; Intel Core i5-13400, 10 physical cores (heterogeneous P/E cores), 16 logical processors; Windows Balanced power scheme.
- Rust 1.75.0, `x86_64-pc-windows-msvc`, LLVM 17.0.6; Rayon 1.10.0, rayon-core 1.12.1. Release profile: opt-level 3, one codegen unit, fat LTO, panic abort. Default `keywords` feature enabled.
- Starting commit: `8e35125b7374d7be443d3355be4fa323a9140034`, plus the pre-existing source changes described above.
- Corpus: `benchmarks/ChangFeng_Simp.txt`, **3,288,254 UTF-8 bytes, 1,130,220 Unicode scalar values** (the character count includes punctuation/whitespace, not only CJK). SHA-256: `431a1e9ff0264977a16d21f7012ad5d24f6ed4e481e5cd82ca2a48524b7ea308`. The complete novel produces 234,068 delimiter ranges.
- Requested excerpt sizes: 500, 1,000, 2,000, 3,000, 4,000, 5,000, 6,000, 7,000, 8,000, 9,000, 10,000, 15,000, 20,000, 50,000 bytes. Three starting positions per size, at 11%, 47%, and 83% of the available start-offset range. Starts advance to a character boundary; ends round **up** to a valid boundary, yielding target through target+2 bytes. This deliberately exercises `>=` at each threshold. Samples can begin/end within a sentence or word; all candidates receive the exact same slices.
- One full-novel sample. Two separate fixed Rayon pools: 16 workers (matching available logical CPUs) and four workers. Runs execute serially, never concurrently. Dictionary creation, pool initialization, corpus reading, compilation, and output checking happen outside timers.
- Every combination uses thresholds 1,000 through 9,000, plus forced parallel (0) and forced sequential (`usize::MAX`). Forced sequential retains the same multi-worker pool: it selects the actual serial algorithm rather than measuring Rayon with a one-worker pool. Threshold selection uses a relaxed atomic load **only in the isolated benchmark copy**; production retains a constant. This small load overhead is common to all cases; tiny timing differences should not be interpreted as significant.
- Three warmup calls **per candidate per case**, then **12 measured iterations per candidate**. Candidate order rotates and reverses across measured iterations. Excerpt and operation order stay fixed. Timing uses `Instant`; the input is passed through `black_box`. Output allocation and destruction are timed; assertions, file I/O, and CSV writing are outside each timed interval.
- Each candidate's complete result is asserted equal to the forced sequential result before timing, including token/tag order and joined delimiters. The benchmark does not substitute a different segmenter or reimplement either branch.
- Median, P10, P90, median absolute deviation (MAD), and input-byte throughput are retained for every individual case. Percentiles use linear interpolation. MiB/s = actual input bytes / median seconds / 1,048,576. Percentile spread describes these 12 observations; it is not a confidence interval.
- A first exploratory excerpt pass rounded boundaries down. Final excerpt results replace that pass with upward-rounded boundaries; full-novel measurements are retained because their input is unchanged. Compilation and each measurement phase were separate. No CPU affinity, fixed frequency, scheduler isolation, or system-wide idle guarantee was imposed. Results characterize this machine and power policy, not every deployment.

## Reproduction and artifacts

Run from the repository root in PowerShell:

```powershell
python benchmarks/prepare_threshold_bench.py
cargo build --release --offline --manifest-path target/threshold-bench/Cargo.toml --bin threshold_benchmark
& target/threshold-bench/target/release/threshold_benchmark.exe 16 benchmarks/ChangFeng_Simp.txt benchmarks/threshold-16.csv
& target/threshold-bench/target/release/threshold_benchmark.exe 4 benchmarks/ChangFeng_Simp.txt benchmarks/threshold-4.csv
python benchmarks/analyze_threshold.py
```

The preparation script copies source/data into ignored `target/threshold-bench`, removes unrelated workspace members, and adds the benchmark-only threshold selector and entrypoint. The isolated lockfile is pruned offline; no dependency version differs from the repository lockfile. It never changes the production library's visibility/API. `excerpts` as a final executable argument skips the full novel for an excerpt-only rerun. Do not analyze a CSV while its writer is still running.

- [Detailed comparison and crossover tables](threshold-tables.md): candidate scores, sequential/parallel timing and throughput at every size, and full-novel candidate variation.
- [Per-case statistics](threshold-summary.csv): actual bytes, range counts, sample count, median, P10/P90, MAD, MiB/s.
- [16-worker raw measurements](threshold-16.csv) and [four-worker raw measurements](threshold-4.csv): every measured interval.
- [Harness](threshold_harness.rs), [isolated-build preparation](prepare_threshold_bench.py), and [analysis](analyze_threshold.py): standard-library-only benchmark tooling; no new runtime dependencies.
