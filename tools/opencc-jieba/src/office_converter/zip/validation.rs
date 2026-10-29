//! Mandatory validation at the random-access parser boundary.
use super::result::{invalid, ZipResult};
use super::spec::FixedSizeBlock;
use super::types::{ZipCentralEntryBlock, ZipFileData, ZipLocalEntryBlock};
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;

// Keep the original header bytes: ZIP 3's extra-field parser replaces file_name_raw
// with UnicodePath, so comparing the parsed names alone misses raw-name cloaking.
fn variable_fields(
    reader: &mut impl Read,
    name_len: u16,
    extra_len: u16,
) -> ZipResult<(Vec<u8>, Vec<u8>)> {
    let mut name = vec![0; usize::from(name_len)];
    let mut extra = vec![0; usize::from(extra_len)];
    reader.read_exact(&mut name)?;
    reader.read_exact(&mut extra)?;
    Ok((name, extra))
}

fn extra_field(extra: &[u8], wanted: u16) -> ZipResult<Option<&[u8]>> {
    let mut remaining = extra;
    let mut path = None;
    while !remaining.is_empty() {
        if remaining.len() < 4 {
            return Err(invalid!("Truncated ZIP extra field"));
        }
        let kind = u16::from_le_bytes([remaining[0], remaining[1]]);
        let len = usize::from(u16::from_le_bytes([remaining[2], remaining[3]]));
        let field = remaining
            .get(4..4 + len)
            .ok_or(invalid!("Truncated ZIP extra field"))?;
        if kind == wanted {
            if path.is_some() {
                return Err(invalid!("Duplicate ZIP identity extra field"));
            }
            path = Some(field);
        }
        remaining = &remaining[4 + len..];
    }
    Ok(path)
}

pub(super) fn unicode_path(extra: &[u8]) -> ZipResult<Option<&[u8]>> {
    let path = extra_field(extra, 0x7075)?;
    if path.map_or(false, |p| p.len() < 5) {
        return Err(invalid!("Invalid UnicodePath extra field"));
    }
    Ok(path)
}

fn local_sizes(local: &ZipLocalEntryBlock, extra: &[u8]) -> ZipResult<(u64, u64)> {
    let mut zip64 = extra_field(extra, 1)?.unwrap_or_default();
    let mut size = |raw| -> ZipResult<u64> {
        if raw != u32::MAX {
            return Ok(u64::from(raw));
        }
        let bytes = zip64.get(..8).ok_or(invalid!("Missing ZIP64 local size"))?;
        let value = u64::from_le_bytes(bytes.try_into().unwrap());
        zip64 = &zip64[8..];
        Ok(value)
    };
    let uncompressed = size(local.uncompressed_size)?;
    let compressed = size(local.compressed_size)?;
    Ok((compressed, uncompressed))
}

pub(super) fn validate_headers<R: Read + Seek>(
    reader: &mut R,
    files: &[ZipFileData],
    directory_start: u64,
) -> ZipResult<()> {
    let mut names = HashSet::new();
    let mut spans = Vec::with_capacity(files.len());
    for file in files {
        if !names.insert(&file.file_name) {
            return Err(invalid!("Duplicate ZIP filename"));
        }
        reader.seek(SeekFrom::Start(file.central_header_start))?;
        let central = ZipCentralEntryBlock::parse(reader)?;
        let (central_name, central_extra) =
            variable_fields(reader, central.file_name_length, central.extra_field_length)?;
        reader.seek(SeekFrom::Start(file.header_start))?;
        let local = ZipLocalEntryBlock::parse(reader)?;
        let (local_name, local_extra) =
            variable_fields(reader, local.file_name_length, local.extra_field_length)?;
        if local_name != central_name {
            return Err(invalid!(
                "ZIP header inconsistency: LFH/CDFH filename mismatch"
            ));
        }
        if unicode_path(&local_extra)? != unicode_path(&central_extra)? {
            return Err(invalid!("ZIP header inconsistency: UnicodePath mismatch"));
        }
        // Compression option bits may differ, but identity/decoding flags must agree.
        if local.compression_method != central.compression_method
            || (local.flags ^ central.flags) & (1 | 8 | 0x800) != 0
        {
            return Err(invalid!(
                "ZIP header inconsistency: compression or flags mismatch"
            ));
        }
        let (compressed, uncompressed) = local_sizes(&local, &local_extra)?;
        let pending_descriptor = local.flags & 8 != 0;
        if (local.crc32 != central.crc32 && (!pending_descriptor || local.crc32 != 0))
            || (compressed != file.compressed_size && (!pending_descriptor || compressed != 0))
            || (uncompressed != file.uncompressed_size
                && (!pending_descriptor || uncompressed != 0))
        {
            return Err(invalid!("ZIP header inconsistency: CRC or size mismatch"));
        }
        let data_start = file
            .header_start
            .checked_add(size_of::<ZipLocalEntryBlock>() as u64)
            .and_then(|v| v.checked_add(u64::from(local.file_name_length)))
            .and_then(|v| v.checked_add(u64::from(local.extra_field_length)))
            .ok_or(invalid!("ZIP local header offset overflow"))?;
        let end = data_start
            .checked_add(file.compressed_size)
            .ok_or(invalid!("ZIP data offset overflow"))?;
        if end > directory_start {
            return Err(invalid!("ZIP entry overlaps central directory"));
        }
        spans.push((file.header_start, end));
        // Cache only after validation, for subsequent by_index/by_index_raw calls.
        let _ = file.data_start.set(data_start);
    }
    spans.sort_unstable();
    if spans.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(invalid!("Overlapping ZIP entries"));
    }
    Ok(())
}

/// Portable package path policy (including Windows drive-relative paths and ADS).
pub(super) fn unsafe_path(name: &str) -> bool {
    name.starts_with(['/', '\\'])
        || name.contains(['\0', ':'])
        || name.split(['/', '\\']).any(|part| part == "..")
}
