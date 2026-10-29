# Incremental Jieba joining

Branch: `refactor/incremental-jieba-join`; baseline commit: `2d97651`.

## Reproduction

Provide `benchmarks/ChangFeng_Simp.txt` locally (not embedded or added by this
change). SHA-256:
`431a1e9ff0264977a16d21f7012ad5d24f6ed4e481e5cd82ca2a48524b7ea308`.
The file contains 3,288,254 UTF-8 bytes and 1,130,220 Unicode scalar values.

Windows PowerShell, Rust 1.75.0, Intel Core i5-13400 (10 cores, 16 logical
processors), repository release profile (opt-level 3, fat LTO, one codegen unit):

```powershell
$env:RAYON_NUM_THREADS = '8'
cargo run --release -j 1 --example jieba_join_bench -- 15
cargo test --release -j 1 -p opencc-jieba-rs --lib --tests
cargo test --release -j 1 --test jieba_join -- --include-ignored
```

The example reads the file and initializes dictionaries before measuring. It
checks equality, warms each implementation three times per HMM mode, then
alternates measurement order over 15 samples. Each sample measures one complete
call, including output destruction; file I/O and printing are excluded.
Allocation counters are disabled during timing. A separate warmed call counts
allocation/reallocation requests and requested bytes across all Rayon workers.
Requested bytes are cumulative, including reallocations, not live or peak memory.
Worker scratch-buffer reuse and scheduling can vary the allocation totals.

## Baseline captured before implementation changes

Both paths still used the original implementation at this point:

| HMM | Expression median (ms) | Public join median (ms) | Public join allocations | Public join requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Off | 43.942 | 43.418 | 1,082,724 | 175,715,455 |
| On | 38.393 | 38.791 | 1,017,662 | 170,784,607 |

## Final paired comparison

The original expression remains available as the control. These measurements
were taken after all builds/tests finished, with no concurrent compilation:

| HMM | Original median ms (min–max) | Incremental median ms (min–max) | Time reduction | Speed ratio |
| --- | ---: | ---: | ---: | ---: |
| Off | 42.270 (39.800–45.081) | 24.751 (23.070–26.752) | 41.4% | 1.71× |
| On | 37.976 (36.759–40.731) | 24.309 (22.684–24.975) | 36.0% | 1.56× |

| HMM | Original allocation calls | Incremental allocation calls | Original requested bytes | Incremental requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Off | 1,080,625 | 579,439 | 175,404,623 | 112,849,191 |
| On | 1,016,260 | 579,439 | 171,073,639 | 112,785,399 |

Allocation calls fell 46.4% / 43.0%, and cumulative requested bytes fell
35.7% / 34.1% (HMM off / on). Timings describe this fixture, space delimiter,
machine, and worker count; they are not a universal speedup claim. The paired
control avoids attributing between-run timing drift to the implementation.

## Validation

Release library/integration tests: 94 unit tests and 72 existing integration
tests passed. The final join test run passed all three new integration tests,
including the external novel fixture (169 distinct passing tests in total).
The novel is checked in both HMM modes with `" "`, `"/"`, `"|"`, `""`, and
`"🙂分隔"` delimiters. `git diff --check` passed.

An initial broad Cargo test build encountered an archive permission error.
Retrying with `-j 1 --lib --tests` succeeded; unrelated example executables
were not part of that retry. The benchmark example built and ran successfully.

## Implementation and limits

`jieba_cut()` and the tokenizer remain unchanged. The join uses the same inclusive
ranges, HMM setting, and 1,000-byte parallel threshold. Below the threshold,
borrowed words append directly to the final string. Above it, indexed Rayon
collection produces one joined string and a token-presence flag per chunk;
ordered assembly inserts separators only between chunks containing tokens.
Presence is independent of string length, preserving empty-token semantics.

The tokenizer still returns a temporary `Vec<Token>` per chunk; this focused
refactor eliminates per-token owned strings and the global `Vec<String>`, rather
than changing the tokenizer API. Ranges, parallel chunk strings, and tokenizer
scratch allocations remain. No process peak-memory measurement was taken.

Regression coverage compares against `jieba_cut(...).join(...)` for empty and
short inputs, threshold boundaries, long unbroken text, punctuation-only chunks,
whitespace, NUL, Unicode, both HMM modes, five delimiters, and one/four-worker
Rayon pools. A unit test explicitly checks empty-token presence. The ignored
novel test requires the external fixture and is run explicitly in release mode.
