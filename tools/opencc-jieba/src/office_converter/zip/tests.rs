use super::write::SimpleFileOptions;
use super::{CompressionMethod, ZipArchive, ZipWriter};
use std::io::{Cursor, Read, Write};

fn archive(method: CompressionMethod, zip64: bool) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "safe.txt",
            SimpleFileOptions::default()
                .compression_method(method)
                .large_file(zip64),
        )
        .unwrap();
    writer.write_all(b"inert test data").unwrap();
    writer.finish().unwrap().into_inner()
}

fn central(bytes: &[u8]) -> usize {
    bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap()
}

#[test]
fn stored_deflate_zip64_and_raw_copy_round_trip() {
    for method in [CompressionMethod::Stored, CompressionMethod::Deflated] {
        for zip64 in [false, true] {
            let bytes = archive(method, zip64);
            let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer.raw_copy_file(input.by_index(0).unwrap()).unwrap();
            let bytes = writer.finish().unwrap().into_inner();
            let mut output = ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut file = output.by_index(0).unwrap();
            assert_eq!(file.name(), "safe.txt");
            assert_eq!(file.compression(), method);
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            assert_eq!(content, b"inert test data");
        }
    }
}

#[test]
fn mismatched_raw_names_fail_before_inspection_or_raw_copy() {
    for change_local in [false, true] {
        let mut bytes = archive(CompressionMethod::Stored, false);
        let start = if change_local {
            30
        } else {
            central(&bytes) + 46
        };
        bytes[start..start + 8].copy_from_slice(b"evil.txt");
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let copied = (|| {
            let mut input = ZipArchive::new(Cursor::new(bytes))?;
            let file = input.by_index_raw(0)?;
            writer.raw_copy_file(file)
        })();
        assert!(copied
            .unwrap_err()
            .to_string()
            .contains("filename mismatch"));
        let output = writer.finish().unwrap().into_inner();
        assert_eq!(ZipArchive::new(Cursor::new(output)).unwrap().len(), 0);
    }
}

#[test]
fn every_truncated_prefix_fails_safely() {
    let bytes = archive(CompressionMethod::Deflated, false);
    for len in 0..bytes.len() {
        assert!(
            ZipArchive::new(Cursor::new(&bytes[..len])).is_err(),
            "length {len}"
        );
    }
}

#[test]
fn inconsistent_metadata_and_out_of_bounds_payload_fail() {
    let original = archive(CompressionMethod::Stored, false);
    for offset in [6, 8, 14, 18, 22, 26, 28] {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        assert!(
            ZipArchive::new(Cursor::new(bytes)).is_err(),
            "LFH byte {offset}"
        );
    }
    let mut bytes = original;
    let cd = central(&bytes);
    bytes[18..22].copy_from_slice(&100_000u32.to_le_bytes());
    bytes[cd + 20..cd + 24].copy_from_slice(&100_000u32.to_le_bytes());
    assert!(ZipArchive::new(Cursor::new(bytes))
        .unwrap_err()
        .to_string()
        .contains("overlaps central"));
}

#[test]
fn unicode_path_and_malformed_extra_field_validation() {
    use super::validation::unicode_path;
    let valid = [0x75, 0x70, 6, 0, 1, 0, 0, 0, 0, b'x'];
    assert_eq!(unicode_path(&valid).unwrap(), Some(&valid[4..]));
    let duplicate = [valid, valid].concat();
    assert!(unicode_path(&duplicate).is_err());
    for bytes in [&valid[..3], &valid[..9], &[0x75, 0x70, 0, 0][..]] {
        assert!(unicode_path(bytes).is_err());
    }
}

fn with_unicode_path(local_text: Option<&str>, central_text: Option<&str>) -> Vec<u8> {
    fn extra(text: Option<&str>) -> Vec<u8> {
        let Some(text) = text else { return Vec::new() };
        let mut bytes = vec![0x75, 0x70];
        bytes.extend_from_slice(&((text.len() + 5) as u16).to_le_bytes());
        bytes.push(1);
        bytes.extend_from_slice(&crc32fast::hash(b"safe.txt").to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }
    let mut bytes = archive(CompressionMethod::Stored, false);
    let local = extra(local_text);
    let central_extra = extra(central_text);
    bytes[28..30].copy_from_slice(&(local.len() as u16).to_le_bytes());
    bytes.splice(38..38, local);
    let cd = central(&bytes);
    bytes[cd + 30..cd + 32].copy_from_slice(&(central_extra.len() as u16).to_le_bytes());
    bytes.splice(cd + 54..cd + 54, central_extra);
    let eocd = bytes.windows(4).position(|b| b == b"PK\x05\x06").unwrap();
    bytes[eocd + 12..eocd + 16].copy_from_slice(&((eocd - cd) as u32).to_le_bytes());
    bytes[eocd + 16..eocd + 20].copy_from_slice(&(cd as u32).to_le_bytes());
    bytes
}

#[test]
fn unicode_override_cannot_hide_raw_or_effective_header_mismatch() {
    let good = with_unicode_path(Some("文件.txt"), Some("文件.txt"));
    let mut archive = ZipArchive::new(Cursor::new(&good)).unwrap();
    assert_eq!(archive.by_index(0).unwrap().name(), "文件.txt");
    let mut bad = good;
    bad[30] = b'x'; // Both Unicode names still agree; original names must also agree.
    assert!(ZipArchive::new(Cursor::new(bad)).is_err());
    for (local, central) in [
        (Some("other.txt"), Some("文件.txt")),
        (None, Some("文件.txt")),
        (Some("文件.txt"), None),
    ] {
        let bytes = with_unicode_path(local, central);
        assert!(ZipArchive::new(Cursor::new(bytes))
            .unwrap_err()
            .to_string()
            .contains("UnicodePath mismatch"));
    }
}

#[test]
fn data_descriptor_placeholders_remain_compatible() {
    let mut bytes = archive(CompressionMethod::Stored, false);
    let cd = central(&bytes);
    let mut descriptor = b"PK\x07\x08".to_vec();
    descriptor.extend_from_slice(&bytes[14..26]);
    bytes[6] |= 8;
    bytes[14..26].fill(0);
    bytes[cd + 8] |= 8;
    bytes.splice(cd..cd, descriptor);
    let eocd = bytes.len() - 22;
    bytes[eocd + 16..eocd + 20].copy_from_slice(&((cd + 16) as u32).to_le_bytes());
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut content = String::new();
    archive
        .by_index(0)
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "inert test data");
}

#[test]
fn zip64_size_mismatch_and_duplicate_names_are_rejected() {
    let mut bytes = archive(CompressionMethod::Stored, true);
    // First ZIP64 local extra-field value is the uncompressed size.
    bytes[42] ^= 1;
    assert!(ZipArchive::new(Cursor::new(bytes))
        .unwrap_err()
        .to_string()
        .contains("size mismatch"));

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in ["first.txt", "other.txt"] {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    let original = writer.finish().unwrap().into_inner();
    let mut duplicate = original.clone();
    let positions: Vec<_> = duplicate
        .windows(9)
        .enumerate()
        .filter(|(_, v)| *v == b"other.txt")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(positions.len(), 2);
    for pos in positions {
        duplicate[pos..pos + 9].copy_from_slice(b"first.txt");
    }
    assert!(ZipArchive::new(Cursor::new(duplicate))
        .unwrap_err()
        .to_string()
        .contains("Duplicate"));

    let mut overlap = original;
    let cd = central(&overlap);
    // Extend the first payload into the second LFH while keeping both headers equal.
    overlap[18..22].copy_from_slice(&21u32.to_le_bytes());
    overlap[cd + 20..cd + 24].copy_from_slice(&21u32.to_le_bytes());
    assert!(ZipArchive::new(Cursor::new(overlap))
        .unwrap_err()
        .to_string()
        .contains("Overlapping"));
}
