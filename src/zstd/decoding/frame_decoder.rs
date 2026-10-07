//! Framedecoder is the main low-level struct users interact with to decode zstd frames
//!
//! Zstandard compressed data is made of one or more frames. Each frame is independent and can be
//! decompressed independently of other frames. This module contains structures
//! and utilities that can be used to decode a frame.

use super::frame;
use crate::zstd::decoding;
use crate::zstd::decoding::errors::FrameDecoderError;
use crate::zstd::decoding::scratch::DecoderScratch;
use std::io::Read;
use std::vec::Vec;

/// The default maximum window size, in bytes, that a [FrameDecoder] accepts.
///
/// Defaults to 100mb to bound allocation for malformed or hostile frames. The
/// spec permits far larger windows; this internal decoder keeps a fixed limit.
pub const DEFAULT_MAX_WINDOW_SIZE: u64 = 1024 * 1024 * 100;

/// Low level Zstandard decoder that can be used to decompress frames with fine control over when and how many bytes are decoded.
///
/// This decoder is able to decode frames only partially and gives control
/// over how many bytes/blocks will be decoded at a time (so you don't have to decode a 10GB file into memory all at once).
/// It reads bytes as needed from a provided source and can be read from to collect partial results.
pub struct FrameDecoder {
    state: Option<FrameDecoderState>,
}

struct FrameDecoderState {
    pub frame_header: frame::FrameHeader,
    decoder_scratch: DecoderScratch,
    frame_finished: bool,
    check_sum: Option<u32>,
}

pub enum BlockDecodingStrategy {
    UptoBytes(usize),
}

impl FrameDecoderState {
    fn new(
        source: impl Read,
        max_window_size: u64,
    ) -> Result<FrameDecoderState, FrameDecoderError> {
        let (frame, _) = frame::read_frame_header(source)?;
        let window_size = frame.window_size()?;
        Self::check_window_size(window_size, max_window_size)?;
        Ok(FrameDecoderState {
            frame_header: frame,
            frame_finished: false,
            decoder_scratch: DecoderScratch::new(window_size as usize),
            check_sum: None,
        })
    }

    fn reset(&mut self, source: impl Read, max_window_size: u64) -> Result<(), FrameDecoderError> {
        let (frame_header, _) = frame::read_frame_header(source)?;
        let window_size = frame_header.window_size()?;
        Self::check_window_size(window_size, max_window_size)?;

        self.frame_header = frame_header;
        self.frame_finished = false;
        self.decoder_scratch.reset(window_size as usize);
        self.check_sum = None;
        Ok(())
    }

    /// Reject a frame whose declared window exceeds the limit, before allocation.
    fn check_window_size(window_size: u64, max_window_size: u64) -> Result<(), FrameDecoderError> {
        if window_size > max_window_size {
            return Err(FrameDecoderError::WindowSizeTooBig {
                requested: window_size,
                max: max_window_size,
            });
        }
        Ok(())
    }
}

impl FrameDecoder {
    /// This will create a new decoder without allocating anything yet.
    /// init() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    pub fn new() -> FrameDecoder {
        FrameDecoder { state: None }
    }

    /// init() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    ///
    /// Note that all bytes currently in the decodebuffer from any previous frame will be lost. Collect them with collect_into()
    pub fn init(&mut self, source: impl Read) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        let state = match &mut self.state {
            Some(s) => {
                s.reset(source, DEFAULT_MAX_WINDOW_SIZE)?;
                s
            }
            None => {
                self.state = Some(FrameDecoderState::new(source, DEFAULT_MAX_WINDOW_SIZE)?);
                self.state.as_mut().unwrap()
            }
        };
        if let Some(dict_id) = state.frame_header.dictionary_id() {
            // The entry points never provide a Zstandard decoding dictionary.
            return Err(err::DictNotProvided { dict_id });
        }
        Ok(())
    }

    /// Returns the uncompressed content size declared by the current frame.
    ///
    /// Returns `Some(size)` when the frame contains a frame content size (FCS),
    /// including `Some(0)` for an explicitly declared empty frame. Returns `None`
    /// when the decoder has not been initialized or the frame does not contain an
    /// FCS.
    pub(crate) fn content_size(&self) -> Option<u64> {
        self.state
            .as_ref()
            .and_then(|state| state.frame_header.frame_content_size_opt())
    }

    /// Whether the current frames last block has been decoded yet
    /// If this returns true you can call the drain* functions to get all content
    /// (the read() function will drain automatically if this returns true)
    pub fn is_finished(&self) -> bool {
        let state = match &self.state {
            None => return true,
            Some(s) => s,
        };
        if state.frame_header.descriptor.content_checksum_flag() {
            state.frame_finished && state.check_sum.is_some()
        } else {
            state.frame_finished
        }
    }

    /// Decodes blocks from a reader. It requires that the framedecoder has been initialized first.
    /// The Strategy influences how many blocks will be decoded before the function returns
    /// The byte target bounds how much output is decoded per call.
    pub fn decode_blocks(
        &mut self,
        mut source: impl Read,
        strat: BlockDecodingStrategy,
    ) -> Result<bool, FrameDecoderError> {
        use FrameDecoderError as err;
        let state = self.state.as_mut().ok_or(err::NotYetInitialized)?;

        let mut block_dec = decoding::block_decoder::new();

        let buffer_size_before = state.decoder_scratch.buffer.len();
        loop {
            let (block_header, _) = block_dec
                .read_block_header(&mut source)
                .map_err(err::FailedToReadBlockHeader)?;

            block_dec
                .decode_block_content(&block_header, &mut state.decoder_scratch, &mut source)
                .map_err(err::FailedToReadBlockBody)?;

            if block_header.last_block {
                state.frame_finished = true;
                if state.frame_header.descriptor.content_checksum_flag() {
                    let mut chksum = [0u8; 4];
                    source
                        .read_exact(&mut chksum)
                        .map_err(err::FailedToReadChecksum)?;
                    let chksum = u32::from_le_bytes(chksum);
                    state.check_sum = Some(chksum);
                }
                break;
            }

            match strat {
                BlockDecodingStrategy::UptoBytes(n) => {
                    if state.decoder_scratch.buffer.len() - buffer_size_before >= n {
                        break;
                    }
                }
            }
        }

        Ok(state.frame_finished)
    }

    /// Append decoded bytes to output, retaining the history window until finished.
    pub fn collect_into(&mut self, output: &mut Vec<u8>) {
        let finished = self.is_finished();
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let buffer = &mut state.decoder_scratch.buffer;
        let amount = if finished {
            buffer.len()
        } else {
            buffer.can_drain_to_window_size().unwrap_or(0)
        };
        buffer.drain_into(output, amount);
    }
}
