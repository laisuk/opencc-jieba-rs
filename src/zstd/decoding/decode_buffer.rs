use std::io::{Error, Read};
use std::vec::Vec;

use super::ringbuffer::RingBuffer;
use crate::zstd::decoding::errors::DecodeBufferError;

pub struct DecodeBuffer {
    buffer: RingBuffer,

    pub window_size: usize,
    total_output_counter: u64,
}

impl Read for DecodeBuffer {
    fn read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let max_amount = self.can_drain_to_window_size().unwrap_or(0);
        let amount = max_amount.min(target.len());

        let mut written = 0;
        self.drain_to(amount, |buf| {
            target[written..][..buf.len()].copy_from_slice(buf);
            written += buf.len();
            (buf.len(), Ok(()))
        })?;
        Ok(amount)
    }
}

impl DecodeBuffer {
    pub fn new(window_size: usize) -> DecodeBuffer {
        DecodeBuffer {
            buffer: RingBuffer::new(),
            window_size,
            total_output_counter: 0,
        }
    }

    pub fn reset(&mut self, window_size: usize) {
        self.window_size = window_size;
        self.buffer.clear();
        self.buffer.reserve(self.window_size);
        self.total_output_counter = 0;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn extend_and_fill(&mut self, fill_with: u8, fill_length: usize) {
        self.buffer.extend_and_fill(fill_with, fill_length);
    }

    pub fn extend_from_reader<R: Read>(
        &mut self,
        read: R,
        fill_length: usize,
    ) -> Result<(), Error> {
        self.buffer.extend_from_reader(read, fill_length)
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend(data);
        self.total_output_counter += data.len() as u64;
    }

    pub fn repeat(&mut self, offset: usize, match_length: usize) -> Result<(), DecodeBufferError> {
        if offset > self.buffer.len() {
            self.repeat_from_dict(offset)
        } else {
            let buf_len = self.buffer.len();
            let start_idx = buf_len - offset;
            let end_idx = start_idx + match_length;

            self.buffer.reserve(match_length);
            if end_idx > buf_len {
                // We need to copy in chunks.
                self.repeat_in_chunks(offset, match_length, start_idx);
            } else {
                // can just copy parts of the existing buffer
                // SAFETY: Requirements checked:
                // 1. start_idx + match_length must be <= self.buffer.len()
                //      We know that:
                //      1. start_idx = self.buffer.len() - offset
                //      2. end_idx = start_idx + match_length
                //      3. end_idx <= self.buffer.len()
                //      Thus follows: start_idx + match_length <= self.buffer.len()
                //
                // 2. explicitly reserved enough memory for the whole match_length
                unsafe {
                    self.buffer
                        .extend_from_within_unchecked(start_idx, match_length)
                };
            }

            self.total_output_counter += match_length as u64;
            Ok(())
        }
    }

    fn repeat_in_chunks(&mut self, offset: usize, match_length: usize, start_idx: usize) {
        // We have at max offset bytes in one chunk, the last one can be smaller
        let mut start_idx = start_idx;
        let mut copied_counter_left = match_length;
        // TODO this can  be optimized further I think.
        // Each time we copy a chunk we have a repetiton of length 'offset', so we can copy offset * iteration many bytes from start_idx
        while copied_counter_left > 0 {
            let chunksize = usize::min(offset, copied_counter_left);

            // SAFETY: Requirements checked:
            // 1. start_idx + chunksize must be <= self.buffer.len()
            //      We know that:
            //      1. start_idx starts at buffer.len() - offset
            //      2. chunksize <= offset (== offset for each iteration but the last, and match_length modulo offset in the last iteration)
            //      3. the buffer grows by offset many bytes each iteration but the last
            //      4. start_idx is increased by the same amount as the buffer grows each iteration
            //
            //      Thus follows: start_idx + chunksize == self.buffer.len() in each iteration but the last, where match_length modulo offset == chunksize < offset
            //          Meaning: start_idx + chunksize <= self.buffer.len()
            //
            // 2. explicitly reserved enough memory for the whole match_length
            unsafe {
                self.buffer
                    .extend_from_within_unchecked(start_idx, chunksize)
            };
            copied_counter_left -= chunksize;
            start_idx += chunksize;
        }
    }

    #[cold]
    fn repeat_from_dict(&mut self, offset: usize) -> Result<(), DecodeBufferError> {
        if self.total_output_counter <= self.window_size as u64 {
            // No decoding dictionary is supplied by the entry points. Preserve
            // the original error for an offset beyond the available history.
            Err(DecodeBufferError::NotEnoughBytesInDictionary {
                got: 0,
                need: offset - self.buffer.len(),
            })
        } else {
            Err(DecodeBufferError::OffsetTooBig {
                offset,
                buf_len: self.buffer.len(),
            })
        }
    }

    /// Check if and how many bytes can currently be drawn from the buffer
    pub fn can_drain_to_window_size(&self) -> Option<usize> {
        if self.buffer.len() > self.window_size {
            Some(self.buffer.len() - self.window_size)
        } else {
            None
        }
    }

    /// Append and remove the requested number of decoded bytes without a temporary vector.
    pub fn drain_into(&mut self, output: &mut Vec<u8>, amount: usize) {
        // Reserve once for both slices when the ring buffer wraps.
        output.reserve(amount);
        self.drain_to(amount, |buf| {
            output.extend_from_slice(buf);
            (buf.len(), Ok(()))
        })
        .expect("appending to a Vec cannot return an I/O error");
    }

    /// Semantics of write_bytes:
    /// Should dump as many of the provided bytes as possible to whatever sink until no bytes are left or an error is encountered
    /// Return how many bytes have actually been dumped to the sink.
    fn drain_to(
        &mut self,
        amount: usize,
        mut write_bytes: impl FnMut(&[u8]) -> (usize, Result<(), Error>),
    ) -> Result<usize, Error> {
        if amount == 0 {
            return Ok(0);
        }

        struct DrainGuard<'a> {
            buffer: &'a mut RingBuffer,
            amount: usize,
        }

        impl Drop for DrainGuard<'_> {
            fn drop(&mut self) {
                if self.amount != 0 {
                    self.buffer.drop_first_n(self.amount);
                }
            }
        }

        let mut drain_guard = DrainGuard {
            buffer: &mut self.buffer,
            amount: 0,
        };

        let (slice1, slice2) = drain_guard.buffer.as_slices();
        let n1 = slice1.len().min(amount);
        let n2 = slice2.len().min(amount - n1);

        if n1 != 0 {
            let (written1, res1) = write_bytes(&slice1[..n1]);
            drain_guard.amount += written1;

            // Apparently this is what clippy thinks is the best way of expressing this
            res1?;

            // Only if the first call to write_bytes was not a partial write we can continue with slice2
            // Partial writes SHOULD never happen without res1 being an error, but let's just protect against it anyway.
            if written1 == n1 && n2 != 0 {
                let (written2, res2) = write_bytes(&slice2[..n2]);
                drain_guard.amount += written2;

                // Apparently this is what clippy thinks is the best way of expressing this
                res2?;
            }
        }

        let amount_written = drain_guard.amount;
        // Make sure we don't accidentally drop `DrainGuard` earlier.
        drop(drain_guard);

        Ok(amount_written)
    }
}
