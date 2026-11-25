use super::scratch::DecoderScratch;
use crate::zstd::decoding::errors::ExecuteSequencesError;

/// Take the provided decoder and execute the sequences stored within
pub fn execute_sequences(
    scratch: &mut DecoderScratch,
    block_limit: usize,
) -> Result<(), ExecuteSequencesError> {
    let size_error = || ExecuteSequencesError::BlockSizeLimitExceeded { max: block_limit };
    // Every literal is emitted once, including those after the final sequence.
    let mut match_budget = block_limit
        .checked_sub(scratch.literals_buffer.len())
        .ok_or_else(size_error)?;
    let mut literals_used = 0usize;

    for &seq in &scratch.sequences {
        match_budget = match_budget
            .checked_sub(seq.ml as usize)
            .ok_or_else(size_error)?;
        let end = literals_used
            .checked_add(seq.ll as usize)
            .ok_or_else(size_error)?;
        if end > scratch.literals_buffer.len() {
            return Err(ExecuteSequencesError::NotEnoughBytesForSequence {
                wanted: end,
                have: scratch.literals_buffer.len(),
            });
        }
        let offset = do_offset_history(seq.of, seq.ll, &mut scratch.offset_hist)
            .ok_or(ExecuteSequencesError::ZeroOffset)?;
        if end != literals_used {
            scratch
                .buffer
                .push(&scratch.literals_buffer[literals_used..end]);
            literals_used = end;
        }
        if seq.ml != 0 {
            scratch.buffer.repeat(offset as usize, seq.ml as usize)?;
        }
    }
    if literals_used < scratch.literals_buffer.len() {
        scratch
            .buffer
            .push(&scratch.literals_buffer[literals_used..]);
    }
    Ok(())
}

/// Update the most recently used offsets to reflect the provided offset value, and return the
/// "actual" offset needed because offsets are not stored in a raw way, some transformations are needed
/// before you get a functional number.
fn do_offset_history(offset_value: u32, lit_len: u32, scratch: &mut [u32; 3]) -> Option<u32> {
    let actual_offset = if lit_len > 0 {
        match offset_value {
            1..=3 => scratch[offset_value as usize - 1],
            _ => {
                //new offset
                offset_value.checked_sub(3)?
            }
        }
    } else {
        match offset_value {
            1..=2 => scratch[offset_value as usize],
            3 => scratch[0].checked_sub(1)?,
            _ => {
                //new offset
                offset_value.checked_sub(3)?
            }
        }
    };

    if actual_offset == 0 {
        return None;
    }

    //update history
    if lit_len > 0 {
        match offset_value {
            1 => {
                //nothing
            }
            2 => {
                scratch[1] = scratch[0];
                scratch[0] = actual_offset;
            }
            _ => {
                scratch[2] = scratch[1];
                scratch[1] = scratch[0];
                scratch[0] = actual_offset;
            }
        }
    } else {
        match offset_value {
            1 => {
                scratch[1] = scratch[0];
                scratch[0] = actual_offset;
            }
            2 => {
                scratch[2] = scratch[1];
                scratch[1] = scratch[0];
                scratch[0] = actual_offset;
            }
            _ => {
                scratch[2] = scratch[1];
                scratch[1] = scratch[0];
                scratch[0] = actual_offset;
            }
        }
    }

    Some(actual_offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zstd::blocks::sequence_section::Sequence;

    #[test]
    fn match_budget_handles_cumulative_output_before_copying() {
        let mut scratch = DecoderScratch::new(1024);
        scratch.literals_buffer.push(b'x');
        scratch.sequences.push(Sequence {
            ll: 1,
            ml: 600,
            of: 1,
        });
        scratch.sequences.push(Sequence {
            ll: 0,
            ml: 600,
            of: 4,
        });
        assert!(matches!(
            execute_sequences(&mut scratch, 1024),
            Err(ExecuteSequencesError::BlockSizeLimitExceeded { max: 1024 })
        ));
        assert_eq!(scratch.buffer.len(), 601);
    }

    #[test]
    fn repeat_offset_zero_and_underflow_return_errors() {
        for recent in [0, 1] {
            let mut scratch = DecoderScratch::new(1024);
            scratch.offset_hist[0] = recent;
            scratch.sequences.push(Sequence {
                ll: 0,
                ml: 3,
                of: 3,
            });
            assert!(matches!(
                execute_sequences(&mut scratch, 1024),
                Err(ExecuteSequencesError::ZeroOffset)
            ));
            assert_eq!(scratch.offset_hist[0], recent);
            assert_eq!(scratch.buffer.len(), 0);
        }
    }
}
