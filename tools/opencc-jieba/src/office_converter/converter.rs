//! # OfficeConverter Module
//!
//! This module provides the [`OfficeConverter`] type, which performs **Chinese text
//! conversion inside Office and EPUB documents** using the [`OpenCC`] engine.
//!
//! ## Supported Formats
//! - `.docx` (Word)
//! - `.xlsx` (Excel)
//! - `.pptx` (PowerPoint, including slides & notes)
//! - `.odt`, `.ods`, `.odp` (LibreOffice / OpenDocument)
//! - `.epub` (E-books)
//!
//! ## Features
//! - Extracts ZIP-based archives into a temp folder
//! - Runs OpenCC conversion (`s2t`, `t2s`, etc.)
//! - Optionally converts punctuation
//! - Optionally preserves original fonts (masking and restoring)
//! - Repackages into a valid archive
//!   - EPUBs ensure `mimetype` is the first entry and stored uncompressed
//!
//! ## Example
//! ```rust,no_run
//! use opencc_fmmseg::OpenCC;
//! use crate::converter::OfficeConverter;
//!
//! let opencc = OpenCC::new("s2t").unwrap();
//! let result = OfficeConverter::convert(
//!     "input.docx",
//!     "output.docx",
//!     "docx",
//!     &opencc,
//!     "s2t",
//!     true,   // punctuation
//!     true    // keep fonts
//! ).unwrap();
//!
//! assert!(result.success);
//! println!("{}", result.message);
//! ```
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::{
    write::{ExtendedFileOptions, FileOptions},
    CompressionMethod, ZipArchive, ZipWriter,
};

use opencc_jieba_rs::OpenCC;

/// Result of a document conversion operation.
///
/// Holds a success flag and an explanatory message.
pub struct ConversionResult {
    pub success: bool,
    pub message: Box<str>,
}

/// Converter for Office and EPUB documents.
///
/// Provides functionality to:
/// - Extract archives (`.docx`, `.xlsx`, `.pptx`, `.odt`, `.ods`, `.odp`, `.epub`)
/// - Run OpenCC text conversion
/// - Optionally preserve fonts
/// - Repackage into a valid output archive
pub struct OfficeConverter;

/// Precompiled regex patterns for extracting fonts
/// from XML or XHTML text inside supported formats.
struct FontPatterns {
    docx: Regex,
    xlsx: Regex,
    pptx: Regex,
    odt: Regex,
    epub: Regex,
}

impl FontPatterns {
    /// Initialize all regex patterns once.
    fn new() -> Self {
        Self {
            docx: Regex::new(r#"(w:(?:eastAsia|ascii|hAnsi|cs)=")(.*?)(")"#).unwrap(),
            xlsx: Regex::new(r#"(val=")(.*?)(")"#).unwrap(),
            pptx: Regex::new(r#"(typeface=")(.*?)(")"#).unwrap(),
            odt: Regex::new(r#"((?:style:font-name(?:-asian|-complex)?|svg:font-family|style:name)=['"])([^'"]+)(['"])"#).unwrap(),
            epub: Regex::new(r#"(font-family\s*:\s*)([^;"']+)"#).unwrap(),
        }
    }

    /// Return the regex for a given Office/EPUB format, if available.
    fn get_pattern(&self, format: &str) -> Option<&Regex> {
        match format {
            "docx" => Some(&self.docx),
            "xlsx" => Some(&self.xlsx),
            "pptx" => Some(&self.pptx),
            "odt" | "ods" | "odp" => Some(&self.odt),
            "epub" => Some(&self.epub),
            _ => None,
        }
    }
}

// Use thread_local for regex patterns to avoid recompilation
thread_local! {
    /// Thread-local storage for font regex patterns.
    ///
    /// Ensures regexes are compiled once per thread, avoiding
    /// global lock contention and reallocation overhead.
    static FONT_PATTERNS: FontPatterns = FontPatterns::new();
}

impl OfficeConverter {
    /// Convert an input Office/EPUB file using OpenCC and output a new archive.
    ///
    /// # Arguments
    /// - `input_path`: Path to the input `.docx`, `.xlsx`, `.pptx`, `.odt`, `.ods`, `.odp`, or `.epub` file
    /// - `output_path`: Path to save the converted file
    /// - `format`: File format string (e.g. `"docx"`, `"epub"`)
    /// - `helper`: Reference to an `OpenCC` instance
    /// - `config`: OpenCC conversion config (e.g. `"s2t"`)
    /// - `punctuation`: Whether to convert punctuation
    /// - `keep_font`: Whether to preserve original font declarations
    ///
    /// # Returns
    /// A `ConversionResult` with success flag and status message.
    pub fn convert(
        input_path: &str,
        output_path: &str,
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<ConversionResult> {
        let temp_dir = tempdir()?;
        let temp_path = temp_dir.path();

        // Extract archive into temp dir
        Self::extract_archive(input_path, temp_path)?;

        // Convert targeted XML/text files
        Self::convert_xml_files(format, temp_path, helper, config, punctuation, keep_font)?;

        // Repackage into output file
        Self::create_output_archive(format, temp_path, input_path, output_path)?;

        Ok(ConversionResult {
            success: true,
            message: "✅ Conversion completed.".into(),
        })
    }

    /// Extract the given ZIP-based archive into a temp folder.
    ///
    /// Rejects unsafe paths (zip-slip, parent/root dirs).
    fn extract_archive(input_path: &str, temp_path: &Path) -> io::Result<()> {
        let file = File::open(input_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let raw_name = entry.name().replace('\\', "/");
            let rel_path = Path::new(&raw_name);

            // Security check: reject zip-slip & roots
            if Self::is_unsafe_path(rel_path) {
                continue;
            }

            let out_path = temp_path.join(rel_path);

            if entry.is_dir() || raw_name.ends_with('/') {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out_path)?;
                io::copy(&mut entry, &mut out_file)?;
            }
        }
        Ok(())
    }

    /// Detect unsafe paths (zip-slip, `..`, root dirs).
    fn is_unsafe_path(path: &Path) -> bool {
        path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    }

    /// Convert targeted XML/text files inside the extracted archive.
    ///
    /// Uses buffered I/O for performance.
    fn convert_xml_files(
        format: &str,
        temp_path: &Path,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<()> {
        let xml_paths = get_target_xml_paths(format, temp_path);

        for xml_file in xml_paths {
            if !xml_file.exists() || !xml_file.is_file() {
                continue;
            }

            // Use buffered I/O for better performance on large files
            let mut content = String::new();
            {
                let file = File::open(&xml_file)?;
                let mut reader = BufReader::new(file);
                reader.read_to_string(&mut content)?;
            }

            let mut font_map = HashMap::new();
            if keep_font {
                Self::mask_font(&mut content, format, &mut font_map);
            }

            let mut converted = helper.convert(&content, config, punctuation);

            if keep_font {
                // More efficient string replacement using drain pattern
                for (marker, original) in font_map {
                    converted = converted.replace(&marker, &original);
                }
            }

            // Use buffered writer
            {
                let file = File::create(&xml_file)?;
                let mut writer = BufWriter::new(file);
                writer.write_all(converted.as_bytes())?;
                writer.flush()?;
            }
        }
        Ok(())
    }

    /// Create an output ZIP archive from a temp folder.
    fn create_output_archive(
        format: &str,
        temp_path: &Path,
        input_path: &str,
        output_path: &str,
    ) -> io::Result<()> {
        let out_path = Path::new(output_path);
        let in_path_abs = Path::new(input_path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(input_path));
        let out_path_abs = out_path
            .canonicalize()
            .unwrap_or_else(|_| out_path.to_path_buf());

        if out_path_abs == in_path_abs {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output_path must differ from input_path",
            ));
        }

        replace_with_temp(out_path, |zip_writer| {
            Self::write_zip_contents(format, temp_path, zip_writer)
        })
    }

    /// Write all files back into a ZIP archive.
    ///
    /// For EPUB, ensures `mimetype` is first and uncompressed.
    fn write_zip_contents(
        format: &str,
        temp_path: &Path,
        zip_writer: &mut ZipWriter<File>,
    ) -> io::Result<()> {
        // EPUB: ensure 'mimetype' is first and stored
        if format.eq_ignore_ascii_case("epub") {
            Self::write_mimetype_first(temp_path, zip_writer)?;
        }

        // Write all other files
        for entry in WalkDir::new(temp_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            let rel = path
                .strip_prefix(temp_path)
                .map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("strip_prefix failed: {}", e))
                })?
                .to_string_lossy()
                .replace('\\', "/");

            // Skip mimetype for EPUB (already written)
            if format.eq_ignore_ascii_case("epub") && rel == "mimetype" {
                continue;
            }

            Self::write_file_to_zip(path, &rel, zip_writer)?;
        }
        Ok(())
    }

    /// Write EPUB `mimetype` file first (stored, no compression).
    fn write_mimetype_first(temp_path: &Path, zip_writer: &mut ZipWriter<File>) -> io::Result<()> {
        let mimetype_path = temp_path.join("mimetype");
        if mimetype_path.exists() && mimetype_path.is_file() {
            let mut buf = Vec::new();
            File::open(&mimetype_path)?.read_to_end(&mut buf)?;
            let opts: FileOptions<'_, ExtendedFileOptions> =
                FileOptions::default().compression_method(CompressionMethod::Stored);
            zip_writer.start_file("mimetype", opts)?;
            zip_writer.write_all(&buf)?;
        }
        Ok(())
    }

    /// Write a file into the ZIP with proper compression.
    fn write_file_to_zip(
        file_path: &Path,
        relative_path: &str,
        zip_writer: &mut ZipWriter<File>,
    ) -> io::Result<()> {
        let mut buffer = Vec::new();
        File::open(file_path)?.read_to_end(&mut buffer)?;

        let method = if relative_path == "mimetype" {
            CompressionMethod::Stored
        } else {
            CompressionMethod::Deflated
        };

        let options: FileOptions<'_, ExtendedFileOptions> =
            FileOptions::default().compression_method(method);

        zip_writer.start_file(relative_path, options)?;
        zip_writer.write_all(&buffer)?;
        Ok(())
    }

    /// Replace font declarations with markers, storing originals in `font_map`.
    fn mask_font(xml: &mut String, format: &str, font_map: &mut HashMap<String, String>) {
        FONT_PATTERNS.with(|patterns| {
            if let Some(re) = patterns.get_pattern(format) {
                let mut counter = 0;
                let mut result_str = String::with_capacity(xml.len() + xml.len() / 10); // Pre-allocate with buffer
                let mut last_end = 0;

                for caps in re.captures_iter(xml) {
                    let marker = format!("__F_O_N_T_{}__", counter);
                    counter += 1;
                    font_map.insert(marker.clone(), caps[2].to_string());

                    let mat = caps.get(0).unwrap();
                    result_str.push_str(&xml[last_end..mat.start()]);
                    result_str.push_str(&caps[1]);
                    result_str.push_str(&marker);

                    if caps.len() > 3 {
                        result_str.push_str(&caps[3]);
                    }
                    last_end = mat.end();
                }
                result_str.push_str(&xml[last_end..]);
                *xml = result_str;
            }
        });
    }
}

/* ---------- Helper Functions ---------- */

/// Remove an existing file if present, handling Windows read-only flags.
fn remove_existing_file(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("output_path is a directory: {:?}", path),
        ));
    }

    // Handle read-only files on Windows
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        #[cfg(windows)]
        if perms.readonly() {
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
        }
    }
    fs::remove_file(path)
}

/// Write to a temp file then atomically replace the final path.
///
/// Ensures no partial/corrupted output if interrupted.
fn replace_with_temp(
    final_out: &Path,
    write_zip: impl FnOnce(&mut ZipWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let ext = final_out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("zip");
    let tmp_out = final_out.with_extension(format!("tmp.{}", ext));

    // Clean any stale temp file
    let _ = remove_existing_file(&tmp_out);

    // Create and write temp zip
    {
        let zip_file = File::create(&tmp_out)?;
        let mut zw = ZipWriter::new(zip_file);
        write_zip(&mut zw)?;
        zw.finish()?;
    }

    // Atomic replace: remove existing -> rename temp to final
    remove_existing_file(final_out)?;
    fs::rename(&tmp_out, final_out)
}

/// Get target XML files for a given format inside extracted archive.
fn get_target_xml_paths(format: &str, base_dir: &Path) -> Vec<PathBuf> {
    match format {
        "docx" => vec![base_dir.join("word/document.xml")],
        "xlsx" => vec![base_dir.join("xl/sharedStrings.xml")],
        "pptx" => get_pptx_files(base_dir),
        "odt" | "ods" | "odp" => vec![base_dir.join("content.xml")],
        "epub" => get_epub_files(base_dir),
        _ => Vec::new(),
    }
}

/// Collect all PPTX slide/notes `.xml` files (excluding `.rels`).
fn get_pptx_files(base_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for dir in ["ppt/slides", "ppt/notesSlides"] {
        let root = base_dir.join(dir);
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("xml")
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".rels"))
                    .unwrap_or(false)
            {
                result.push(path.to_path_buf());
            }
        }
    }
    result
}

/// Collect all EPUB text files (`.xhtml`, `.opf`, `.ncx`, `.html`).
fn get_epub_files(base_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(base_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "xhtml" | "opf" | "ncx" | "html") {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect()
}
