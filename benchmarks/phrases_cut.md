# phrases_cut_impl investigation

Recommendation: **No changes needed**. Leave the library implementation unchanged.
Work was performed on `refactor/incremental-jieba-join`, retaining the preceding
incremental-join changes.

## Source assessment (before experimentation)

`jieba_cut`, `jieba_cut_for_search`, and `jieba_cut_all` share this helper.
`Token<'a>` contains a borrowed `word: &'a str` and Unicode/byte offsets. Its
word borrows the input chunk, not the temporary vector or tokenizer scratch
storage, so borrowing it longer is possible. However, `Vec<String>` cannot
return these borrows: each nonempty token must become an independent owned
string. Empty strings need no backing allocation, but remain vector entries.
Changing to `Cow`, borrowed tokens, or a shared string arena would change the
public contract and is outside scope.

The current `.into_iter().map(|token| token.word.to_owned())` moves token
structures out of their vector and performs one ownership conversion per word.
It does not clone `Token`s, copy strings twice, or recompute their offsets.
Callers that need owned tokens (including the CLI and C API) still need that
ownership; the prior incremental Rust join avoids this path already. No callers
or joining implementations were changed in this investigation.

Allocations come from splitting ranges, tokenizer output vectors and per-thread
segmentation scratch, owned word strings, and result-vector assembly. No CPU
profiler was used, so elapsed-time shares cannot be assigned to these components.
Inspection of the pinned Rayon 1.10 implementation shows that an unindexed
`flat_map_iter().collect::<Vec<_>>()` already accumulates task-local vectors,
reserves their summed lengths, and appends them into the final vector. Strings
move during append; their text is not cloned.

Vector growth and temporary collection bookkeeping are the plausible small
targets. Simply adding another explicit `Vec` or capacity estimate does not
remove the per-word string allocation. Eliminating `Vec<Token>` altogether would
require a tokenizer emission interface for the shared helper's accurate, search,
and full modes. Accurate/search mode already share a private `cut_each` callback,
but it is not the interface consumed by this helper. Such a redesign was not
justified by this focused investigation; its performance remains unmeasured.

## Experiments

The library was not modified. `examples/phrases_cut_bench.rs` compares the current
public `jieba_cut` to two safe candidates using the same tokenizer:

1. **fold_extend:** explicitly extend a task-local vector from each exact-sized
   token iterator, collect the ordered task vectors, reserve the total, and move
   strings into the final result. This mainly restates Rayon's existing strategy
   while making per-chunk size hints available to `extend`.
2. **indexed_chunks:** collect one `Vec<String>` per input range using indexed
   Rayon collection, reserve the exact final length, then append in order. This
   trades result-vector growth for many additional chunk-vector allocations.

Both candidates retain the inclusive ranges and 1,000-byte threshold. The short
path uses a direct loop with per-chunk `extend`. Neither introduces new lifetime
management or production unsafe code. The counting allocator is diagnostic code
following the preceding join benchmark's methodology.

## Method

Fixture: `benchmarks/ChangFeng_Simp.txt`, 3,288,254 bytes, 1,130,220 Unicode scalar
values. SHA-256:
`431a1e9ff0264977a16d21f7012ad5d24f6ed4e481e5cd82ca2a48524b7ea308`.
Windows, Intel Core i5-13400, Rust 1.75.0, repository release profile (fat LTO,
one codegen unit), eight Rayon workers. Three separate runs, each with three
warm-ups and 21 timed samples per candidate and HMM mode, rotating execution
order. File I/O, initialization, equality checks, and console output are excluded.
Output destruction is included. No concurrent builds during measurements.

Allocation counting is disabled during timing. A separate warmed call counts
allocation/reallocation requests and cumulative requested bytes across workers.
These byte totals include reallocations and are not peak or live memory.
Scheduling and thread-local scratch reuse cause small variations.

```powershell
$env:RAYON_NUM_THREADS = '8'
cargo run --release -j 1 --example phrases_cut_bench -- 21
# Repeat the built executable twice:
target/release/examples/phrases_cut_bench.exe 21
target/release/examples/phrases_cut_bench.exe 21
```

## Timing results

All values are median milliseconds within each run:

| Run | HMM | Original | fold_extend | indexed_chunks |
| --- | --- | ---: | ---: | ---: |
| 1 | Off | 35.265 | 34.297 | 43.384 |
| 1 | On | 34.408 | 35.018 | 43.119 |
| 2 | Off | 32.445 | 32.664 | 40.504 |
| 2 | On | 32.193 | 32.273 | 39.963 |
| 3 | Off | 32.962 | 33.035 | 40.651 |
| 3 | On | 32.582 | 32.467 | 39.155 |

Final run, including the separate allocation measurements (Off / On):

| Metric | Original | fold_extend candidate | Difference |
| --- | ---: | ---: | ---: |
| HMM Off median | 32.962 ms | 33.035 ms | 0.073 ms slower (+0.22%) |
| HMM On median | 32.582 ms | 32.467 ms | 0.115 ms faster (-0.35%) |
| Allocation calls Off / On | 1,078,994 / 1,016,286 | 1,078,081 / 1,013,599 | -913 / -2,687 |
| Allocated bytes Off / On | 172,109,582 / 166,815,350 | 172,235,206 / 167,145,590 | +125,624 / +330,240 |

The fold candidate's timing advantage changes sign between runs. Allocation
changes are small and also vary between runs; this is not a meaningful benefit.
In the final run, indexed_chunks used 1,304,527 / 1,240,735 allocation calls
and 140,971,038 / 137,909,022 requested bytes. Its smaller byte total came with
approximately 21–23% slower execution and 21–22% more allocation calls.

## Correctness and decision

The harness directly compares complete `Vec<String>` results for the novel in
both HMM modes. Before timing it also validates both candidates against the
public implementation on empty text, whitespace, punctuation, NUL, Unicode,
999/1,000-byte boundaries, long single chunks and many chunks, using one, four,
and eight-worker pools. All comparisons passed. Token contents and order are
compared directly, without joining or hashing.

The full default-feature package suite passed with
`cargo test --release -j 1 -p opencc-jieba-rs`: 94 unit tests, 74 integration
tests, and 93 documentation tests passed; the external novel test and three
documentation examples were ignored by default. Running
`cargo test --release -j 1 --test jieba_join -- --ignored` also passed the novel
regression, bringing the unit/integration total to 169. `git diff --check`
passed. No new library regression tests were needed because neither candidate
was promoted to the library; their direct comparisons remain in the harness.

There is no demonstrated gain worth additional collection code. No change was
made to `phrases_cut_impl`, its callers, tokenizer, public APIs, dictionaries,
threshold, splitting, or the optimized joining path. Retain the harness and this
report for reproducibility rather than adding an unproven production refactor.
