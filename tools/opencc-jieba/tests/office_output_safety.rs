use std::collections::hash_map::RandomState;
use std::fs;
use std::hash::BuildHasher;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
#[path = "../src/office_converter/zip/mod.rs"]
pub(crate) mod zip;
// Mirror the binary module path for this private-source integration harness.
mod office_converter {
    #[allow(unused_imports)]
    pub(crate) use super::zip;
}
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "opencc-cli-test-{:016x}",
                RandomState::new().hash_one(std::process::id())
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create test directory: {e}"),
            }
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn convert(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_opencc-jieba"))
        .args(["office", "-c", "s2t", "-i"])
        .arg(input)
        .arg("-o")
        .arg(output)
        .output()
        .unwrap()
}

#[test]
fn office_cli_output_lifecycle() {
    let dir = TestDirectory::new();
    let input = dir.0.join("input.docx");
    let output = dir.0.join("output.docx");
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("word/document.xml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all("<w:t>汉语</w:t>".as_bytes()).unwrap();
    let original = zip.finish().unwrap().into_inner();
    fs::write(&input, &original).unwrap();

    for existing in [false, true] {
        if existing {
            fs::write(&output, b"old destination").unwrap();
        }
        let result = convert(&input, &output);
        assert!(result.status.success(), "{:?}", result);
        let mut archive = ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
        let mut text = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "<w:t>漢語</w:t>");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
    }

    let result = convert(&input, &input);
    assert!(!result.status.success());
    assert_eq!(fs::read(&input).unwrap(), original);

    fs::write(&input, b"invalid input ZIP").unwrap();
    fs::write(&output, b"original destination").unwrap();
    let result = convert(&input, &output);
    // This CLI currently reports conversion errors on stderr with a zero exit status.
    assert!(String::from_utf8_lossy(&result.stderr).contains("Error:"));
    assert_eq!(fs::read(&output).unwrap(), b"original destination");
    assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 2);
}
