//! Minimal decoder-only Zstandard implementation for OpenCC embedded resources.
//!
//! Derived from ruzstd 0.9.0 (MIT), by Moritz Borcherding and contributors.
//! Encoder, streaming wrapper, dictionary builder, fuzz/test helpers, and no_std
//! compatibility layers have intentionally been omitted.
//!
//! See `LICENSE-RUZSTD` and `NOTICE.md` in this directory.

mod bit_io;
mod blocks;
mod common;
mod decoding;
mod fse;
mod huff0;

pub(crate) use decoding::errors::FrameDecoderError;
use decoding::{BlockDecodingStrategy, FrameDecoder};

const MAX_FCS_PREALLOC_SIZE: u64 = 64 * 1024 * 1024;

/// Decompresses Zstandard-compressed data into a newly allocated byte vector.
///
/// The frame is decoded incrementally, so the uncompressed size does not need
/// to be known in advance. This allows decoding older or streaming-generated
/// Zstandard frames that do not contain a frame content size (FCS).
///
/// If the frame declares an FCS of at most 64 MiB, it is used only as a
/// preallocation hint for the output vector. Larger or unavailable sizes are
/// ignored, and the output vector grows as needed. The FCS therefore does not
/// affect whether a valid frame can be decoded.
///
/// During incremental decoding, the decoder retains the history window required
/// for Zstandard backreferences and collects output as it becomes available.
pub(crate) fn decompress(input: &[u8]) -> Result<Vec<u8>, FrameDecoderError> {
    let mut decoder = FrameDecoder::new();
    let mut source = input;

    decoder.init(&mut source)?;

    let mut output = decoder
        .content_size()
        .filter(|&size| size <= MAX_FCS_PREALLOC_SIZE)
        .and_then(|size| usize::try_from(size).ok())
        .map(Vec::with_capacity)
        .unwrap_or_default();

    while !decoder.is_finished() {
        decoder.decode_blocks(&mut source, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;

        decoder.collect_into(&mut output);
    }

    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decompress_embedded_matches_original_and_has_valid_fcs() {
        let compressed = include_bytes!("../dictionary_lib/dicts/dictionary.json.zst");
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary.json");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("frame initialization failed");

        let content_size = decoder.content_size();

        let decoded = decompress(compressed).expect("zstd decompression failed");

        // The embedded Zstd artifact contains compact JSON, while the source
        // dictionary JSON is pretty-printed. Compare their parsed JSON values.
        let decoded_json: serde_json::Value =
            serde_json::from_slice(&decoded).expect("decompressed JSON is invalid");

        let expected_json: serde_json::Value =
            serde_json::from_slice(expected).expect("source dictionary JSON is invalid");

        assert_eq!(decoded_json, expected_json);

        // The FCS describes the actual decompressed (compact JSON) payload.
        assert_eq!(
            content_size,
            Some(decoded.len() as u64),
            "embedded Zstd FCS should match the actual decompressed size"
        );
    }

    #[cfg(feature = "dictionary-build")]
    #[test]
    fn decompress_with_fcs_matches_original() {
        let expected = include_bytes!("../dictionary_lib/dicts/dictionary.json");

        // One-shot compression declares the uncompressed frame content size.
        let compressed = zstd::bulk::compress(expected, 3).expect("zstd compression failed");

        let mut decoder = FrameDecoder::new();
        decoder
            .init(compressed.as_slice())
            .expect("frame initialization failed");

        assert_eq!(decoder.content_size(), Some(expected.len() as u64));

        let decoded = decompress(&compressed).expect("zstd decompression failed");

        assert_eq!(decoded.as_slice(), expected);
    }
}

#[cfg(test)]
mod regression_tests;
