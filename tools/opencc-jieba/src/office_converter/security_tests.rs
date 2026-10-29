// Included in converter::tests to exercise the private converter and temp lifecycle.
#[test]
fn all_document_formats_rebuild_and_validate_with_raw_assets() {
    for (format, part) in [
        ("docx", "word/document.xml"),
        ("xlsx", "xl/sharedStrings.xml"),
        ("pptx", "ppt/slides/slide1.xml"),
        ("odt", "content.xml"),
        ("ods", "content.xml"),
        ("odp", "content.xml"),
        ("epub", "OEBPS/chapter.xhtml"),
    ] {
        let mut entries = vec![
            (part, "<t>汉语</t>".as_bytes()),
            ("assets/image.bin", &b"inert binary\0data"[..]),
        ];
        if format == "epub" {
            entries.push(("mimetype", b"application/epub+zip"));
        }
        let input = make_zip(&entries);
        let converter = TextConverter::new(|text: &str| text.replace("汉语", "漢語"));
        let (output, count) =
            OfficeConverter::convert_bytes(&input, format, true, &converter).unwrap();
        assert_eq!(count, 1, "{format}");
        assert_eq!(read_zip_entry(&output, part), "<t>漢語</t>");
        assert_eq!(
            read_zip_entry(&output, "assets/image.bin"),
            "inert binary\0data"
        );
        let mut rebuilt = ZipArchive::new(Cursor::new(&output)).unwrap();
        for i in 0..rebuilt.len() {
            io::copy(&mut rebuilt.by_index(i).unwrap(), &mut io::sink()).unwrap();
        }
        if format == "epub" {
            let first = rebuilt.by_index(0).unwrap();
            assert_eq!(first.name(), "mimetype");
            assert_eq!(first.compression(), CompressionMethod::Stored);
        }
    }
}

#[test]
fn portable_unsafe_paths_are_omitted_from_reconstructed_packages() {
    let unsafe_names = [
        "../outside",
        "a/../../outside",
        "a\\..\\outside",
        "/rooted",
        "\\rooted",
        "C:/absolute",
        "C:relative",
        "safe:stream",
        "nul\0suffix",
        "//server/share",
    ];
    let mut entries = vec![("word/document.xml", &b"<t>text</t>"[..])];
    for name in &unsafe_names {
        entries.push((name, b"inert"));
    }
    let input = make_zip(&entries);
    let converter = TextConverter::new(|text: &str| text.to_owned());
    let (output, _) = OfficeConverter::convert_bytes(&input, "docx", false, &converter).unwrap();
    let archive = ZipArchive::new(Cursor::new(output)).unwrap();
    assert_eq!(archive.len(), 1);
    for name in unsafe_names {
        assert!(OfficeConverter::is_unsafe_zip_name(name), "{name:?}");
    }
}

#[test]
fn header_mismatch_in_target_or_raw_asset_preserves_output() {
    let dir = TestDirectory::new();
    let input_path = dir.0.join("input.docx");
    let output_path = dir.0.join("output.docx");
    let converter = TextConverter::new(|text: &str| text.to_owned());
    for malformed_name in [
        b"word/document.xml".as_slice(),
        b"assets/image.bin".as_slice(),
    ] {
        let mut input = make_zip(&[
            ("word/document.xml", b"<t>text</t>"),
            ("assets/image.bin", b"inert"),
        ]);
        let name = input
            .windows(malformed_name.len())
            .position(|v| v == malformed_name)
            .unwrap();
        input[name] = b'x';
        assert!(OfficeConverter::convert_bytes(&input, "docx", false, &converter).is_err());
        fs::write(&input_path, &input).unwrap();
        for existing in [false, true] {
            if existing {
                fs::write(&output_path, b"original destination").unwrap();
            }
            assert!(OfficeConverter::convert_path_stream(
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "docx",
                false,
                &converter
            )
            .is_err());
            if existing {
                assert_eq!(fs::read(&output_path).unwrap(), b"original destination");
                fs::remove_file(&output_path).unwrap();
            } else {
                assert!(!output_path.exists());
            }
            dir.assert_entries(1);
        }
    }
}
