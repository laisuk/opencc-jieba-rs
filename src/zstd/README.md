# Internal Zstandard decoder

Derived from ruzstd 0.9.0 with selected decoder improvements from v0.9.1
and subsequent upstream safety fixes, adapted for Rust 1.75.
See LICENSE-RUZSTD and NOTICE.md for attribution.

## Entry point and retained dependencies

- `decompress()`: initializes one Zstandard frame, uses a bounded frame content
  size (FCS) only as an output-capacity hint, calls
  `decode_blocks(UptoBytes(...))`, and drains eligible decoded bytes directly
  into the final output vector with `collect_into()`.
- Frames without an FCS remain fully supported. An absent, oversized, or
  otherwise unusable FCS does not prevent decoding; the output vector grows as
  needed.

Decoding requires frame/block parsing, literal and sequence decoding, FSE and
Huffman tables, bit readers, scratch state, and the ring buffer. Collection
retains the history window required for backreferences until the frame finishes,
then drains the remaining output. Repeated entropy tables within a frame remain
supported.

## Preserved behavior

The decoder processes one frame and ignores trailing input. A leading skippable
frame is an error.

Checksum bytes are consumed, and truncated checksums are errors. Checksums are
not validated; the upstream hash feature was never enabled by this crate.
The decoder accepts windows up to 128 MiB and allocates history lazily,
including when reusing a decoder. For frames with an explicit window, raw
and RLE output, compressed bodies, and regenerated compressed output are
bounded by the smaller of the window and 128 KiB. Single-segment frames use
the 128 KiB block limit to preserve tolerance for inaccurate FCS values.
Their declared window still participates in the frame-level window check
and history retention.

OpenCC JSON conversion packs and plaintext Jieba dictionaries are payloads, not Zstandard decoding dictionaries.
No Zstandard decoding dictionary is supplied, so nonzero dictionary IDs return
`DictNotProvided`. Invalid history offsets retain their decoding errors,
including `NotEnoughBytesInDictionary` with zero available bytes where
applicable.

## Frame content size

New dictionary artifacts are generated with one-shot Zstandard compression and
therefore declare an FCS. The decoder uses a valid FCS of at most the configured
preallocation limit only to reserve output capacity; it is not trusted as a
decoding limit or required for compatibility with older streaming-generated
artifacts.

The embedded-artifact regression verifies that the committed Zstandard frame
has an FCS matching its actual decompressed compact JSON payload. Separate regressions
retain coverage for legacy frames without an FCS and for incorrect FCS values
being treated only as allocation hints.

## Decoder fixes

The v0.9.1 backport adds the 128 MiB window limit, per-block size checks,
checked repeat-offset arithmetic, and ring-buffer growth with space for
two blocks beyond a power-of-two history window. The subsequent patches
bound regenerated literals and compressed bodies and reject reserved
sequence compression mode bits. See NOTICE.md for upstream provenance.

The local implementation budgets all literals before sequence execution,
including literals emitted after the last match. Each match is checked
against the remaining budget before it is copied. Huffman output is also
bounded by its declared regenerated literal count. The backport retains
Rust 1.75-compatible operations and the existing dependency set.

## Trimming scope

Removed the dictionary parser/registration/table-copy APIs, general-purpose
frame getters and alternate output APIs, sized-output and `decode_all()` paths,
writer support and writer-only tests, inactive checksum hashing hooks, unused
ring-buffer alternatives, unreachable error variants, and the std I/O
compatibility module. The active decoding algorithms are retained.

The output path drains directly from the history buffer into the final vector,
avoiding the former temporary collection vector and its extra full-output copy.

Regression tests cover embedded data equality and FCS metadata, unknown and
incorrect content sizes, history across collection boundaries, frame/skip
boundaries, checksum consumption, truncation, dictionary IDs, and window
limits. Ring-buffer tests retain a test-only checked wrapper around the
production copy routine.
