# Jieba phase-1 baseline

Baseline: `21d67d439f77e676e3bca33907ecdc5c24b1fbf3`.
The implementation, manifest, lockfile, dictionaries, and build script are
unchanged. The isolated harness is `examples/jieba_baseline.rs`; no dependencies
or public APIs were added. This is a measurement tool, not a performance test
with pass/fail timing thresholds.

## Reproduce (PowerShell, repository root)

```powershell
cargo +1.75.0 build --locked --release --example jieba_baseline --target-dir target/jieba-baseline
$env:RAYON_NUM_THREADS = '1'
& ./target/jieba-baseline/release/examples/jieba_baseline.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/baseline-1-thread.csv
$env:RAYON_NUM_THREADS = '16'
& ./target/jieba-baseline/release/examples/jieba_baseline.exe |
    Set-Content -Encoding utf8 benchmarks/jieba/baseline-16-threads.csv
```

Run each invocation to completion without concurrent builds, tests, benchmarks,
or heavy applications. Environment assignments apply to the current shell;
restore your previous `RAYON_NUM_THREADS` setting afterward. On other platforms,
use the corresponding executable name without `.exe`. The harness uses only
the standard library and this package's existing dependencies. It compiles the
unchanged private Jieba, hash, and decoder modules into the example, using the
package's generated HMM tables and embedded dictionary bytes. The real library
is linked separately for public wrapper measurements; these have independent
private-module caches. Do not use total process RSS to infer a production
instance's memory footprint from this combined binary.

## Method

- Rust 1.75.0, repository release profile: opt-level 3, one codegen unit, fat
  LTO, panic abort, symbols stripped; no `target-cpu=native` override.
- Each steady-state case warms once, doubles its batch until it takes at least
  40 ms, then measures seven batches of that fixed size. CSV times are the
  minimum, median, and maximum **batch mean nanoseconds per operation**, not
  individual-operation percentiles or confidence intervals. `iterations_per_sample`
  records the batch size. Inputs/results pass through `std::hint::black_box`.
- Normal cases include result destruction, including owned strings. Constructor
  cases explicitly ending in `_drop` include destruction. This differs from
  cold constructor rows, which retain the instance after timing construction.
- `dict.*` cases use seven independent operations with state prepared before
  each measurement; setup and final-state destruction are excluded. Internal
  clones/replacements/destruction performed by the measured API are included.
  All mutations start from equivalent state and do not accumulate across samples.
  Sub-microsecond dictionary operations are especially sensitive to timer noise.
- A disabled global allocation counter remains installed during timed runs.
  After timing, one separate warmed invocation counts allocation/reallocation
  calls and cumulative requested bytes (including new realloc size). This is
  **not** peak live memory, retained capacity, physical allocation traffic, or
  deallocation count. Counters are process-wide and can include Rayon workers.
  Timing therefore includes the disabled counter's branch overhead; compare
  future versions with this same harness, not a different allocator setup.
- Five fresh child processes provide `cold1.*` through `cold5.*` rows. Timers
  start inside the child and exclude process launch, DLL loading before main,
  and OS cache flushing. These are fresh process/cache-state observations,
  not cold-storage or simultaneous multi-client startup measurements. Cold
  allocation columns are zero placeholders: **not measured**. Each row is one
  observation; summarize a cold case across the five processes.
- `cold.core.pos_model` uses an empty tokenizer to force compound POS model
  loading. Other cold operations execute in the explicit sequence in `cold()`;
  later operations may benefit from earlier scratch/model initialization.
  Core and public copies have separate static caches. In particular the first
  public keyword calls may also pay POS-model initialization.
- No CPU pinning or frequency lock is imposed. Windows Balanced power mode,
  hybrid cores, scheduler activity, and thermal effects can move measurements.
  One-worker Rayon is not CPU affinity and may still use a worker thread.
  Repeat the complete run when evaluating changes.

## Inputs and scope

Exact UTF-8 fixtures are constants in the harness:

- `simp`: research/software prose and a common Chinese segmentation example.
- `trad`: Taiwan/Hong Kong research, software, banking, and data-security prose.
- `mixed`: Simplified/Traditional Chinese, Latin identifiers, digits, punctuation,
  version numbers, a non-BMP ideograph, and an emoji.
- `short` uses one fixture; `long` repeats the identical fixture 256 times.
  These are reproducible cache-friendly synthetic workloads, **not** a natural
  document corpus or a representative vocabulary distribution. Long TextRank
  has many tokens but few unique vertices/edges; no graph scalability claim
  should be drawn from it.
- The unpunctuated `oov` fixture repeats a Chinese name/new-word/rare-character
  sequence 256 times, stressing HMM/POS fallback and longer blocks. It is
  OOV-oriented, not a proof that every character is absent from the dictionary.
- Both HMM settings are measured for core cut/tag and public cut. Keywords use
  default HMM-off configuration, top-k 10, and no POS filter. Separate top-k
  0/1/100 cases use the short mixed input. Public POS uses HMM on.
- Steady-state conversion uses public `convert(input, "s2t", false)` (the bool
  controls punctuation), with a reused `OpenCC`; this includes conversion maps,
  range splitting, and result allocation, not just Jieba segmentation.
- Custom dictionary measurements cover 1,000 synthetic in-memory entries,
  duplicate reloads, empty transactional loads, explicit/inferred frequency,
  unique/shared tokenizer mutation, and frequency suggestion. Entry construction
  and input formatting are excluded. File I/O, malformed dictionaries, very
  large dictionary scaling, and custom OpenCC conversion packs are not measured.

No dedicated `benches/`, Criterion harness, or existing timed benchmark was
found in the baseline's Cargo manifest, examples, or tests. Existing regression
tests remain the correctness gate; the example's `cfg(not(test))` avoids
duplicating private unit tests in the example test harness.

See `REPORT.md` for measured results, limitations, and the prioritized plan.
