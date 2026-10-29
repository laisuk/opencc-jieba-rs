# Allocation / Cow benchmark for v0.9.0

Run: `cargo run --release --example allocation_bench`

Environment: Windows, rustc 1.75.0, default features, release profile, default Rayon pool. Same working tree and
benchmark harness before/after. Dictionary initialization and three warm-up calls are excluded. Allocation counts
include alloc/alloc_zeroed/realloc across worker threads for one warmed call; bytes are cumulative requested bytes, not
peak live memory. Timing is measured separately with allocation counting disabled. Inputs and outputs are passed through
black_box.

Novel input: the first UTF-8-safe 3,000,000 bytes of `benchmarks/ChangFeng_Simp.txt` (actual 2,999,999 bytes). Short
input: `汉字转换测试`; no-match input: `Hello world 123!`. Both conversions enable punctuation. Short cases run 10,000
iterations; novel cases run 5. These are single-run measurements without confidence intervals; small timing differences
may be noise. CSV files preserve the raw measurements.

| Case            | Before ns/call | After ns/call | Before MiB/s | After MiB/s | Alloc calls before → after | Requested bytes before → after |
|-----------------|---------------:|--------------:|-------------:|------------:|---------------------------:|-------------------------------:|
| short_s2twp     |            633 |           537 |        27.12 |       31.99 |                      7 → 5 |                    1046 → 1010 |
| short_t2twp     |            344 |           265 |        49.83 |       64.78 |                      4 → 3 |                      532 → 514 |
| no_match_s2twp  |           1633 |          1540 |         9.34 |        9.91 |                     11 → 9 |                      944 → 912 |
| no_match_t2twp  |            848 |           805 |        17.99 |       18.95 |                      6 → 5 |                      480 → 464 |
| novel_3mb_s2twp |       48362200 |      41638520 |        59.16 |       68.71 |            910489 → 514137 |          277072901 → 191034327 |
| novel_3mb_t2twp |       25643260 |      23157180 |       111.57 |      123.55 |            457779 → 219989 |           140385245 → 95959862 |

## Hotspots and changes

- `phrases_cut_convert`: eagerly allocated one output String per range plus parallel reduction concatenation buffers. A
  lazy output builder now compares appended spans against the input and creates an owned buffer at the first difference.
  Identity dictionary entries retain precedence over later dictionaries. Parallel processing collects ordered Cow
  chunks, returns Borrowed when all are unchanged, and otherwise concatenates once with exact capacity.
- `convert_rounds`: eagerly allocated output in every round. Unchanged rounds now borrow their input and preserve the
  previous owned buffer.
- `convert_punctuation`: always rebuilt the whole result. The Cow helper borrows when unchanged; its owning adapter
  returns the original String buffer unchanged in that case.
- Internal `st` / `ts` (used by `zho_check`) now return Cow, avoiding unchanged classification intermediates.
- Compatibility normalization now uses a shared lazy scalar mapper that copies the unchanged prefix only on the first
  replacement. Its existing String adapters still allocate once for unchanged public results; this does not eliminate
  the allocation required by the public API.
- `strip_newlines_cow` already had the desired borrowed fast path and was retained.
- Jieba token/range vectors, public Vec<String> segmentation results, DeTofu output buffers, dictionary construction and
  error strings remain separate allocation sources. This is not a claim of allocation-free conversion: public conversion
  APIs still return String, and segmentation still allocates.

## Compatibility and verification

No public signatures changed. Added tests cover borrowed serial/parallel paths, identity dictionary precedence, changed
output, UTF-8 growth, deletion, empty input, single-pass scalar mapping, punctuation directions and owned-buffer reuse.
Existing conversion, normalization, dictionary, public API and documentation tests are also exercised.
