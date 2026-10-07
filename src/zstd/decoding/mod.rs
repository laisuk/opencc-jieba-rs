//! Zstandard frame decoding internals derived from ruzstd 0.9.0.

pub mod errors;
mod frame_decoder;

pub(crate) mod block_decoder;
pub(crate) mod decode_buffer;
pub(crate) mod frame;
pub(crate) mod literals_section_decoder;
mod ringbuffer;
pub(crate) mod scratch;
pub(crate) mod sequence_execution;
pub(crate) mod sequence_section_decoder;

pub(crate) use frame_decoder::{BlockDecodingStrategy, FrameDecoder};
