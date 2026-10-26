# Vendored Jieba performance analysis — phase 1

## Scope and conclusion

Measured baseline: **21d67d439f77e676e3bca33907ecdc5c24b1fbf3** (also the checked-out
HEAD). No implementation, public API, dictionary, dependency, or algorithm was
changed. The new files are a standalone example benchmark and this directory's
methodology, results, and verification log. Two pre-existing untracked workflow
files were left alone.

The strongest opportunity is **reuse of the default TF-IDF table**: public
keyword calls currently rebuild it on every request. Initialization and
dictionary cloning are the next large costs when applications construct or
customize instances frequently. Core segmentation already has low allocation
counts; replacing its trie, DAG, or HMM should be lower priority than eliminating
repeated setup and wrapper temporaries.

These are measured costs and source-backed hypotheses, not measured speedups
of proposed patches. There is no sampling CPU profile or hardware-counter
attribution in this phase. Detailed reproduction, timing definitions, fixtures,
and limitations are in [README.md](README.md). Full measurements are in
[baseline-1-thread.csv](baseline-1-thread.csv) and
[baseline-16-threads.csv](baseline-16-threads.csv): **144 rows each**, comprising
45 fresh-process observations and 99 warmed/prepared cases. Neither run overlapped
with builds or tests. The one-worker run preceded the sixteen-worker run.

## Environment and reproducibility

- Date: 2026-10-26, Asia/Kuala_Lumpur.
- Windows 11 Pro, build 26200, x86_64-pc-windows-msvc.
- Intel Core i5-13400, 10 physical cores / 16 logical processors, approximately
  31.7 GiB visible RAM; Windows Balanced power plan; no affinity/frequency lock.
- rustc 1.75.0 (`82e1608df`, LLVM 17.0.6); locked dependencies; no extra features,
  `RUSTFLAGS`, or `CARGO_ENCODED_RUSTFLAGS`. Repository release profile, fat LTO.
- Explicit `RAYON_NUM_THREADS=1` and `16`. The core and keyword paths are serial;
  changing this setting affects parallel public segmentation/POS/conversion.
- Standard-library timing and a counting System allocator; no new dependencies.
  Allocation counters are disabled during timing and enabled for a separate
  warmed call. They measure cumulative requests, not peak memory.
- A first release build in the normal target directory failed with an archive
  permission error. The isolated `target/jieba-baseline` build succeeded with
  the same compiler/profile and was used for both benchmark runs.

SHA-256 identities (hex, case-insensitive):

| Artifact | SHA-256 |
| --- | --- |
| Cargo.lock | `7340f401129ceaf2509f6aabbc2b62bee95432cf63f646188cf8c11ad8966a7b` |
| Harness | `476bfe5a67a1d26090050843e21fdd8d7576b3d1fd7eb9915b69132a2117a7e8` |
| Measured executable | `a60960a3bc333e3b486c9cc0a644f51a7729dfee2e57c7a8331b235eabd565be` |
| Hans/Hant lexicon .zst | `afae57c63730f0742962aa23fe7fc7bd71698c5d88eacdbf09aeb50eeb805e50` |
| IDF .zst | `fd85f55fb174b83f989cd92dff440b2f6b87ba143eedf34b13d4d3a88c12602e` |
| POS .zst | `1ae100729e7aa7a6e0dc88ce5b8514d7bba813e223604f0fd0ad2f1fae50cc50` |
| hmm.model | `f17790586ac86dd048c8adffed052c4bd2b28ed0682972c1275e59040c0589a7` |
| OpenCC dictionary JSON .zst | `0e7bca83abc3c7d0e3bf2de6106624ec59624c6c936f8b85df0da49b59578f1f` |

## Core steady-state baseline

Times below are **microseconds per operation**, medians of seven batch means
from the one-worker process. Core operations return borrowed token/tag slices;
output-vector destruction is included. TF-IDF/TextRank reuse their extractors,
use HMM off, no POS filter, and top-k 10. Long fixtures repeat the short input
256 times, so their vocabulary is deliberately small.

| Input | UTF-8 bytes | Cut HMM off | Cut HMM on | POS off | POS on | TF-IDF | TextRank |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Simplified short | 141 | 0.605 | 0.633 | 0.689 | 1.566 | 2.183 | 4.033 |
| Traditional short | 141 | 0.590 | 0.610 | 0.683 | 0.710 | 2.335 | 4.817 |
| Mixed short | 118 | 0.576 | 0.765 | 0.768 | 0.886 | 2.627 | 4.862 |
| Simplified long | 36,096 | 144.324 | 155.269 | 179.710 | 393.489 | 229.432 | 259.945 |
| Traditional long | 36,096 | 144.096 | 146.651 | 177.454 | 182.885 | 227.192 | 276.163 |
| Mixed long | 30,208 | 152.440 | 197.784 | 319.559 | 222.191 | 312.851 | 350.758 |
| OOV-oriented continuous Chinese | 12,288 | 40.269 | 60.196 | 61.503 | 272.261 | — | — |

Enabling HMM changes the work and output token count, not just an on/off
overhead: mixed long POS is faster with HMM on because the output differs.
This does not imply that enabling HMM is a semantics-preserving optimization.
The OOV-oriented case makes the compound POS cost much more visible than
dictionary-covered Traditional prose.

Warm short core cut/tag uses **one** allocation for its output. Long core
cut/tag uses **two or three**, caused by output-vector growth, with reusable
internal scratch already warm. For example, long mixed cut HMM off requests
1,376,256 cumulative bytes across three allocations; HMM on requests 589,824
across two. These are not live/peak memory figures.

## Public API and conversion baseline

Times are microseconds; public segmentation owns each returned word and public
POS owns words and tags. Public keyword APIs construct an extractor per call.
Public conversion is a reused instance's `convert(input, "s2t", false)`; its
bool controls punctuation, not an HMM benchmark switch.

| Input | Public cut HMM off, 1 worker | Public cut HMM on, 1 worker | Public POS HMM on, 1 worker | Public TF-IDF, 1 worker | Public TextRank, 1 worker |
| --- | ---: | ---: | ---: | ---: | ---: |
| Simplified short | 1.604 | 1.895 | 3.429 | 39,288.600 | 5.820 |
| Traditional short | 1.601 | 1.757 | 2.673 | 37,389.800 | 6.544 |
| Mixed short | 2.405 | 2.612 | 3.982 | 36,499.850 | 6.710 |
| Simplified long | 400.166 | 406.531 | 891.991 | 37,484.450 | 281.737 |
| Traditional long | 398.028 | 435.552 | 656.066 | 40,801.850 | 324.031 |
| Mixed long | 553.549 | 591.860 | 954.680 | 37,506.950 | 369.781 |

| Long input | Cut HMM on, 1 / 16 workers | POS HMM on, 1 / 16 workers | Conversion, 1 / 16 workers |
| --- | ---: | ---: | ---: |
| Simplified | 406.531 / 322.215 | 891.991 / 435.173 | 504.132 / 162.671 |
| Traditional | 435.552 / 301.181 | 656.066 / 431.636 | 515.019 / 162.659 |
| Mixed | 591.860 / 449.443 | 954.680 / 605.050 | 665.861 / 227.150 |

The source CSV is authoritative for rounding. Short conversion is
1.774 / 1.794 / 2.432 microseconds (Simplified / Traditional / mixed, one worker).
Parallelism helps these long public workloads, especially conversion, but does
not deliver anything near sixteenfold scaling. Public cut still allocates
thousands of strings: Simplified long HMM on has 6,482 allocation requests with
one worker versus 7,804 in the observed sixteen-worker allocation call. Parallel
allocation counts can vary with task splitting; they are not correctness hashes.

The 1,000-byte public parallel threshold is not a proven optimum. These two
settings do not locate the crossover, isolate wrapper work from owned output
costs, or establish multi-client throughput. A threshold/worker-count sweep and
larger real documents are needed before changing scheduling.

## Initialization, first use, and customization

Fresh-process observations below are median/min/max **milliseconds** over five
one-worker child processes. Constructors retain the instance after timing.
They exclude process launch and do not flush OS page caches. Operations execute
in a fixed order: later rows may have warm TLS or model data as explained in
the methodology.

| Fresh-process operation | Median ms | Min–max ms |
| --- | ---: | ---: |
| Core Jieba construction | 162.812 | 147.872–164.288 |
| Core first HMM cut, OOV fixture | 0.036 | 0.030–0.065 |
| Forced first compound POS use | 21.757 | 21.352–23.181 |
| Core first default TF-IDF construction | 57.530 | 51.394–59.277 |
| Core first extraction after TF-IDF construction | 0.032 | 0.027–0.033 |
| Public OpenCC construction | 189.089 | 160.912–197.176 |
| Public first mixed cut | 0.047 | 0.040–0.051 |
| Public first TF-IDF call | 85.989 | 81.879–97.522 |
| Public first TextRank call, after TF-IDF | 0.044 | 0.042–0.046 |

First public TF-IDF can pay both its own IDF initialization and lazy POS model
initialization. The later first TextRank row is not a standalone cold TextRank
latency. The sixteen-worker fresh-process data is also in its CSV; constructor
medians there are 158.048 ms core and 186.745 ms public. Startup variation is
material, so do not compare tiny startup differences as optimizations.

Warm construction/destruction and prepared dictionary operations (one-worker
run, **milliseconds**, unless noted):

| Operation | Median ms | Allocation requests | Cumulative requested bytes |
| --- | ---: | ---: | ---: |
| Core new + drop | 150.776 | 224 | 161,034,685 |
| Public new + drop | 171.240 | 131,147 | 177,810,471 |
| Lexicon Zstd decode + drop | 37.615 | 49 | 43,080,845 |
| Parse decoded lexicon into new Jieba + drop | 119.546 | 175 | 117,953,840 |
| Core clone + drop | 8.673 | 116 | 48,213,384 |
| Default TF-IDF construct + drop, text already decoded | 38.016 | 270,201 | 21,664,546 |
| Default TextRank construct + drop | 0.001858 | 67 | 3,006 |
| Load 1,000 entries into empty core | 0.120 | 25 | 82,071 |
| Load 1,000 new entries into cloned populated core | 3.287 | 4 | 28,264,047 |
| Reload those 1,000 duplicate entries into seeded cloned core | 0.543 | 2 | 63 |
| Public transactional load of 1,000 entries | 11.176 | 119 | 76,477,760 |
| Public transactional empty load | 8.118 | 117 | 48,213,776 |
| Core single explicit-frequency insert after clone | 2.437 | 2 | 28,263,984 |
| Core single inferred-frequency insert after clone | 2.454 | 3 | 28,264,416 |
| Public single insert with shared snapshot held | 9.422 | 119 | 76,477,760 |
| Public single insert into freshly initialized unique instance | 0.001800 | 0 | 0 |
| Core suggest_freq only | 0.000150 | 1 | 432 |

Decode and parse measurements are separate operations with different cache and
destruction effects; their medians need not sum to the constructor median.
Prepared mutations exclude setup and final-state destruction; internal cloning
and replacement drops remain included. In particular **cloning vectors can
discard spare capacity**: the core insert-after-clone rows are not representative
of insertion into a fresh dictionary with spare capacity. The public unique
insertion row demonstrates that distinction, but its single-operation timings
are noisy (1.1–2.4 microseconds in the one-worker run). Frequency inference itself
is cheap for this particular short word; dictionary growth/cloning dominates
the prepared inferred-insert row.

## Findings by component

**Trie and dictionary ingestion — `src/jieba_rs/trie.rs`, `mod.rs`.** The trie
already has a direct BMP root table, open-addressed transitions, two absence
filters, and software lookahead. Every position enumerates matching prefixes
without allocating per-prefix strings. `reserve` exists but the generic
`load_dict` does not call it; records/log-frequency arrays also start empty.
Loading checks `trie.get` before inserting each new word, walking it twice.
Repeated growth/rehashing and duplicate lookup are plausible contributors to
the measured 119.5 ms parse cost, but their individual CPU shares were not
profiled. `finish_load` scans the complete record list even for a small appended
dictionary, explaining a potential fixed cost in the 1,000-duplicate reload.
Tag interning duplicates each distinct tag into the tag array and map, but the
small number of tags makes that a low-priority memory target.

**DAG and route — `sparse_dag.rs`, `mod.rs::calc`.** Edges are packed u64s in
flat CSR storage, include word IDs, and feed cached log frequencies. Route,
decoded characters, edges, and offsets are reused per thread. No per-position
hash maps or nested vector allocations remain. `word_at` scans short ordered
edge lists; it is reused for fallback and search tokens instead of another trie
walk. Dense-prefix dictionaries could increase edge counts and scans, but that
case is not measured here. Scratch retention is capped per vector (4 MiB for
ordinary buffers, 32 MiB for DAG arrays); input bursts beyond the caps will
reallocate on later calls. Retuning those limits requires memory/latency testing,
not a conclusion from these relatively small fixtures.

**HMM — `hmm.rs`.** Built-in emissions use direct indices; four-state Viterbi
uses rolling score arrays, two possible predecessors, and reused backpointers.
It already avoids regex matching and per-token heap allocation. `mod.rs::hmm_cut`
recounts emitted word characters although decoded spans exist upstream; an
internal span callback might avoid that scan. Custom models still use a hash
map and have a separate parser. They are not benchmarked here, so changing the
built-in path cannot be assumed to improve custom models. Preserve exact
floating-point evaluation order and larger-state tie-breaking.

**POS — `posseg.rs`, `mod.rs::tag_each/guess_tag`.** Dictionary word IDs avoid
repeat trie lookup on the normal path. OOV tokens can trigger another Unicode
scan, a second character decode, and compound Viterbi for every occurrence of
the same word. The compound model already uses a transposed transition matrix,
BMP row indices, sparse reachable-state lists, positional predecessor groups,
and TLS scratch. OOV-heavy POS costs 272.3 microseconds versus 60.2 microseconds
for HMM cutting on this workload, motivating per-request reuse of guessed tags.
The dense backpointer table still scales as characters × 256 states; memory
benefits from sparse backpointers are plausible but riskier. First model load
still decompresses text, builds temporary maps/vectors, joins emission lists
with linear searches, then builds indexed rows; the text is retained in a
OnceLock after parsing. Precomputing rows could address startup/retained memory,
but asset generation and exact state-order semantics require a separate audit.

**TF-IDF — `keywords/tfidf.rs`, parent `src/keyword.rs`.** The OnceLock caches
decoded IDF text, not the parsed map. Every parent helper constructs
`TfIdf::default`, allocating approximately 270,000 times and parsing it again.
Reused extraction already streams tags, uses borrowed term keys, linear median
selection during loading, and bounded top-k heaps. It still builds a per-call
term-frequency map, sums frequencies in a second pass, and looks tags up again
for selected output words. These micro-optimizations matter much less than
table reuse. Empty-IDF loading/panic behavior must not be silently changed by
an optimization to the median/load path.

**TextRank — `keywords/textrank.rs`.** The graph is already CSR with precomputed
edge shares and bounded top-k selection. Each request still allocates candidate
IDs, an oversized-by-unique-vocabulary word map/vector, co-occurrence counts,
an edge vector, graph arrays, temporary counting arrays, ranks, and output.
Repeated long inputs reserve by input bytes despite having very few distinct
words. Simplified TextRank grows from 8,836 requested bytes on short text to
500,805 on long text. The token window must retain non-candidate positions.
`Option<usize>` candidate IDs can potentially use a compact sentinel/niche
representation. Do not casually deduplicate/reorient edges or reorder vertices:
the rank updates are in-place for 20 sweeps and edge summation order can affect
floating-point scores and ties.

**Shared keyword configuration — `keywords/mod.rs`.** Every default extractor
clones the stopword BTreeSet and builds a second owned hash lookup: 67 allocations
for TextRank construction alone. `is_keyword` allocates `to_lowercase()` for
each uppercase-bearing candidate occurrence. On long mixed text, core TF-IDF
allocations rise to 1,050 versus 26 for Simplified; TextRank rises to 1,063 versus
38. Source and fixture structure identify 1,024 repeated lowercase temporaries.
Caching candidate eligibility by original word within a request, or sharing
immutable defaults, is more promising than altering Unicode case semantics.
Both extractors check top-k zero only after token counting/graph counting:
mixed short top-k zero still costs 1.557 microseconds / 8 allocations (TF-IDF)
and 2.226 microseconds / 15 allocations (TextRank).

**Parent integration — `src/opencc.rs`, `src/keyword.rs`.** Each public cut
allocates ranges, per-range core vectors, owned token strings, and a merged
result. POS additionally collects an intermediate vector of owned pairs per
range. Conversion allocates per-range strings and reduces/concatenates them in
parallel, although the serial output path already streams into one buffer.
The conversion path computes Unicode token lengths again despite core token
offsets, and can count an unmatched token a second time. Parent keyword wrappers
also convert POS filters into owned strings (not exercised by the no-filter
baseline). These are relevant integration costs, not deficiencies in trie/HMM
algorithms. Required owned output allocations cannot all be removed without an
API change, which is outside scope.

## Prioritized optimization plan (no changes implemented)

Expected benefits below are workload-dependent hypotheses. Complexity is
relative engineering effort, including preserving behavior and validation.

| Priority | Candidate | Expected benefit / measured motivation | Risk | Complexity |
| --- | --- | --- | --- | --- |
| 1 | Share the immutable parsed default IDF table; preserve mutable custom-IDF path and per-extractor config | Very high for public keyword APIs: remove repeated ~38 ms setup and ~21.7 MB cumulative requests; reused extraction takes 2–3 us short / 227–313 us long | Medium: custom dictionary merging, lower median, duplicate records, thread safety, and mutation isolation | Medium |
| 2 | Pre-size built-in trie/records/log-frequency arrays; investigate one-walk insert/update and reusable built-in tokenizer initialization | High for startup-heavy applications: core parse ~120 ms, decode ~38 ms; no steady-state gain promised | Medium for reserve-only; high for shared tokenizer initialization because Arc sharing/snapshot behavior is observable | Medium; sharing is larger |
| 3 | Reduce transactional dictionary clone/growth costs after preserving validation and rollback; preserve useful vector capacity where justified | High for customization: 1,000-entry load ~11.2 ms; clone ~8.7 ms; shared insert ~9.4 ms versus unique insert ~1.8 us | Medium–high: duplicate tags/frequencies, atomic failure, shared snapshots, memory tradeoffs | Medium |
| 4 | Stream core token/tag callbacks into parent outputs; remove intermediate per-range vectors and duplicate token-length scans | Medium–high for public long POS/cut/conversion, low for callers already using core results; thousands of allocations remain in wrappers | Medium: delimiter order, offsets, fallback precedence, parallel ordering, scratch reentrancy | Medium |
| 5 | Share immutable default stopword/config data; memoize per-request candidate eligibility without changing Unicode lowercasing | Medium for short TextRank and repeated mixed-script text: 67 setup allocations, ~1,024 repeated lowercase allocations long | Low–medium: custom config mutations must stay isolated; cache overhead may hurt all-unique text | Low–medium |
| 6 | Memoize OOV POS guesses per request, or reuse decoded spans through internal helpers | Medium on OOV-repeated inputs; OOV POS on ~272 us versus cut ~60 us | Medium: lifetime/scratch rules, custom model scope, cache bounds, exact fallback and tie behavior | Medium |
| 7 | Compact/reuse TextRank candidate and graph scratch arrays; cap uniqueness-based reservations | Medium memory/allocator benefit on long low-vocabulary inputs (~0.5–0.7 MB cumulative requests) | Medium–high: changing hash capacity/order or edge visitation can alter floating-point results | Medium |
| 8 | Consider top-k-zero fast path and carrying TF-IDF tags/frequency totals directly | Low absolute benefit except frequent zero-result requests; current zero path still segments/counts | Low–medium: audit current lazy initialization/panic behavior before bypassing work | Low |
| 9 | Profile trie lookahead/probing, dense-prefix DAG scans, HMM emission/callback scans, POS backpointer layout; sweep Rayon batching/thresholds | Unquantified kernel benefit; existing core is already allocation-efficient; parallel conversion currently gains ~3x on these long fixtures | High for structure/DP changes; medium for scheduling | Medium–high |

Do not implement an empty transactional-load early return merely because it
would save ~8 ms: `OpenCC::jieba` is a public Arc field, and snapshot/identity
behavior needs consideration. Likewise, sharing the default tokenizer changes
the cost and sharing state of later mutations even if read-only outputs match.
Favor reserve-only ingestion changes and IDF-data sharing before either redesign.

## Gates for a later implementation phase

1. Keep this harness, corpus, flags, and allocation instrumentation fixed. Add
   larger diverse documents and high-vocabulary TextRank graphs before making
   graph/scheduling conclusions; add dictionaries with duplicates/prefix chains,
   long uninterrupted Chinese, burst-sized scratch, filtered POS keywords,
   custom HMM/IDF models, and concurrent clients as appropriate to the patch.
2. Measure one candidate at a time against this commit, repeat complete runs,
   and report both throughput and allocation/memory effects. Obtain a sampling
   profile before changing the trie or DP kernels. Do not present the core vs
   public timing ratio as the guaranteed speedup of a patch.
3. Retain existing tests, add focused semantic comparisons only for changed
   behavior-sensitive paths, and compare complete token/tag sequences, Unicode
   and byte offsets, dictionary update/precedence and rollback behavior,
   conversion output, keyword order, and exact weights where the contract needs
   them. Preserve Viterbi ties, NaN/signed-zero ordering, TextRank vertex/edge
   order, 20 in-place updates, and the IDF median definition.
4. Verify Rust 1.75.0 with the locked graph, library tests/doctests, relevant
   feature gates and workspace/C API consumers. Do not introduce dependencies
   or alter the public API for these optimizations without a separate decision.

## Verification

- Rust 1.75 release example build: passed in `target/jieba-baseline`.
- Both complete benchmark invocations: exit 0, 144 data rows each.
- `cargo +1.75.0 test --locked -p opencc-jieba-rs`: **157 unit/integration tests
  and 93 doctests passed**, 3 doctests ignored; no failures. See `test-results.txt`.
- `git diff 21d67d4 -- src Cargo.toml Cargo.lock build.rs` is empty: no measured
  implementation or dependency changes. Existing tests are unchanged.
- No automatic commit was made.
