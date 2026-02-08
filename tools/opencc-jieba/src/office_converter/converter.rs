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
//! use opencc_jieba_rs::OpenCC;
//! use crate::converter::OfficeConverter;
//!
//! let opencc = OpenCC::new();
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
use std::io::{self, BufReader, Cursor, Read, Seek, Write};
use std::path::{Path, PathBuf};

use regex::Regex;
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
        // Delegate to the stream-based implementation (single source of truth).
        Self::convert_path_stream(
            input_path,
            output_path,
            format,
            helper,
            config,
            punctuation,
            keep_font,
        )
    }

    // ------ New In-Memory Functions ------

    /// Convert a ZIP-based Office / EPUB document from in-memory bytes and return the output bytes.
    ///
    /// This function performs a **pure in-memory transformation**:
    /// - No temporary files
    /// - No directory extraction
    /// - No filesystem access
    ///
    /// ### How it works
    /// - The input archive is read **entry by entry**
    /// - Text-based entries (`.xml`, `.xhtml`, etc.) are:
    ///   - decoded as UTF-8
    ///   - converted using OpenCC
    ///   - optionally processed for punctuation and font preservation
    /// - All non-target entries (images, media, fonts, etc.) are copied using
    ///   [`ZipWriter::raw_copy_file`], avoiding decompression and recompression
    ///
    /// ### EPUB compliance
    /// - The `mimetype` entry (if present) is:
    ///   - written **first**
    ///   - written with **stored** compression (no deflate)
    /// - This produces EPUB archives compliant with the EPUB specification
    ///
    /// ### Performance and memory behavior
    /// - Memory usage is **bounded to a single ZIP entry**
    /// - Large documents are safe to process
    /// - Non-text entries are copied with **zero extra allocation**
    ///
    /// ### Returns
    /// - `Vec<u8>`: the converted ZIP archive
    /// - `usize`: number of text fragments converted
    ///
    /// ### Errors
    /// Returns an `io::Error` if:
    /// - the input is not a valid ZIP archive
    /// - a target text entry is not valid UTF-8
    /// - ZIP writing fails
    #[allow(dead_code)]
    pub fn convert_bytes(
        input_zip: &[u8],
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<(Vec<u8>, usize)> {
        let reader = Cursor::new(input_zip);

        let out_cursor = Cursor::new(Vec::<u8>::new());
        let mut z_out = ZipWriter::new(out_cursor);

        let converted_count = Self::convert_zip_stream(
            reader,
            &mut z_out,
            format,
            helper,
            config,
            punctuation,
            keep_font,
        )?;

        // finish() returns the inner writer (Cursor<Vec<u8>>)
        let out_cursor = z_out.finish()?;
        Ok((out_cursor.into_inner(), converted_count))
    }

    /// Convert an Office / EPUB document from an input file path to an output file path
    /// using the streaming ZIP conversion core.
    ///
    /// This function provides a **filesystem-based façade** over the in-memory ZIP
    /// processing engine:
    /// - The input archive is read as a ZIP stream
    /// - Entries are processed **one by one**
    /// - The output archive is written atomically to the destination path
    ///
    /// ### How it works
    /// - Opens the input file as a ZIP archive
    /// - Writes the output ZIP using a temporary file, then atomically replaces
    ///   `output_path` on success
    /// - Internally delegates all conversion logic to the same core used by
    ///   [`convert_bytes`]
    ///
    /// ### Safety and correctness
    /// - `output_path` must differ from `input_path` (validated)
    /// - No partial or corrupted output is produced if an error occurs
    /// - EPUB output enforces correct `mimetype` handling
    ///
    /// ### Performance characteristics
    /// - No directory extraction
    /// - No temporary folders
    /// - Memory usage is bounded to a single ZIP entry
    /// - Non-text entries are copied using raw ZIP passthrough when possible
    ///
    /// ### Returns
    /// A [`ConversionResult`] indicating success and a human-readable status message.
    ///
    /// ### Errors
    /// Returns an `io::Error` if:
    /// - the input file cannot be opened
    /// - the output file cannot be written
    /// - the input is not a valid ZIP-based Office / EPUB document
    pub fn convert_path_stream(
        input_path: &str,
        output_path: &str,
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<ConversionResult> {
        let in_path_abs = Path::new(input_path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(input_path));
        let out_path = Path::new(output_path);

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
            let file = File::open(input_path)?;
            let reader = BufReader::new(file);

            Self::convert_zip_stream(
                reader,
                zip_writer,
                format,
                helper,
                config,
                punctuation,
                keep_font,
            )?;

            Ok(())
        })?;

        Ok(ConversionResult {
            success: true,
            message: "✅ Conversion completed.".into(),
        })
    }

    /// Core ZIP-to-ZIP conversion engine.
    ///
    /// Reads a ZIP-based Office / EPUB document from `reader` and writes a new ZIP
    /// archive to `z_out`, processing entries **one by one**.
    ///
    /// This function is the **single conversion core** used by both
    /// [`convert_bytes`] and [`convert_path_stream`].
    ///
    /// ### Processing model
    /// - The input archive is iterated **entry by entry**
    /// - For each entry:
    ///   - **Target text entries** (`.xml`, `.xhtml`, etc.) are:
    ///     - decoded as UTF-8
    ///     - converted using OpenCC
    ///     - optionally processed for punctuation and font preservation
    ///     - recompressed and written to the output archive
    ///   - **Non-target entries** (images, media, fonts, etc.) are copied using
    ///     [`ZipWriter::raw_copy_file`], preserving:
    ///       - compressed data
    ///       - CRC
    ///       - compression method
    ///       - ZIP metadata
    ///
    /// ### EPUB handling
    /// - If `format` is `"epub"` and a `mimetype` entry exists:
    ///   - it is written **first**
    ///   - it is written with **stored** compression (no deflate)
    /// - All other entries follow, preserving EPUB specification requirements
    ///
    /// ### Performance and memory guarantees
    /// - No directory extraction
    /// - No temporary folders
    /// - Memory usage is **bounded to a single ZIP entry**
    /// - Large documents are safe to process
    ///
    /// ### Returns
    /// The number of text fragments that were converted.
    ///
    /// ### Errors
    /// Returns an `io::Error` if:
    /// - the input is not a valid ZIP archive
    /// - a target text entry is not valid UTF-8
    /// - ZIP writing fails
    fn convert_zip_stream<R, W>(
        reader: R,
        z_out: &mut ZipWriter<W>,
        format: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
        keep_font: bool,
    ) -> io::Result<usize>
    where
        R: Read + Seek,
        W: Write + Seek,
    {
        let mut zin = ZipArchive::new(reader)?;
        let mut converted_count = 0;

        // -----------------------------
        // EPUB: write `mimetype` first
        // -----------------------------
        let mut mimetype_index: Option<usize> = None;
        if format.eq_ignore_ascii_case("epub") {
            mimetype_index = Self::find_mimetype_index(&mut zin)?;

            if let Some(mi) = mimetype_index {
                let mut entry = zin.by_index(mi)?;
                let name = entry.name().replace('\\', "/");

                // Security: reject zip-slip & roots
                if !Self::is_unsafe_path(Path::new(&name)) && !entry.is_dir() && name == "mimetype"
                {
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf)?;

                    let opts: FileOptions<'_, ExtendedFileOptions> =
                        FileOptions::default().compression_method(CompressionMethod::Stored);

                    z_out.start_file("mimetype", opts)?;
                    z_out.write_all(&buf)?;
                }
            }
        }

        // -----------------------------
        // Write all other entries
        // -----------------------------
        for i in 0..zin.len() {
            // Skip `mimetype` for EPUB (already written first)
            if format.eq_ignore_ascii_case("epub") && mimetype_index == Some(i) {
                continue;
            }

            let mut entry = zin.by_index(i)?;
            let name = entry.name().replace('\\', "/");

            // Security: reject zip-slip & roots
            if Self::is_unsafe_path(Path::new(&name)) {
                continue;
            }

            if entry.is_dir() || name.ends_with('/') {
                let opts: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(CompressionMethod::Stored);
                z_out.add_directory(name, opts)?;
                continue;
            }

            if Self::is_target_entry(format, &name) {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;

                let mut font_map = HashMap::new();
                if keep_font {
                    Self::mask_font(&mut content, format, &mut font_map);
                }

                let mut converted = helper.convert(&content, config, punctuation);

                if keep_font {
                    for (marker, original) in font_map {
                        converted = converted.replace(&marker, &original);
                    }
                }

                let opts: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(CompressionMethod::Deflated);

                z_out.start_file(name, opts)?;
                z_out.write_all(converted.as_bytes())?;
                converted_count += 1;
            } else {
                z_out.raw_copy_file(entry)?;
            }
        }

        Ok(converted_count)
    }

    /// Determine if a ZIP entry name should be converted for the given format.
    ///
    /// This replaces the previous tempdir-based path discovery.
    fn is_target_entry(format: &str, name: &str) -> bool {
        match format {
            "docx" => name == "word/document.xml",
            "xlsx" => name == "xl/sharedStrings.xml",
            "pptx" => {
                // Convert ppt/slides/*.xml and ppt/notesSlides/*.xml, excluding *.rels
                let is_xml = name.ends_with(".xml");
                let is_rels = name.ends_with(".rels");
                let in_slides = name.starts_with("ppt/slides/");
                let in_notes = name.starts_with("ppt/notesSlides/");
                is_xml && !is_rels && (in_slides || in_notes)
            }
            "odt" | "ods" | "odp" => name == "content.xml",
            "epub" => {
                // Convert XHTML/OPF/NCX/HTML anywhere
                let lower = name.to_ascii_lowercase();
                lower.ends_with(".xhtml")
                    || lower.ends_with(".opf")
                    || lower.ends_with(".ncx")
                    || lower.ends_with(".html")
            }
            _ => false,
        }
    }

    /// Find the ZIP entry index for `mimetype` (EPUB), if present.
    fn find_mimetype_index<R: Read + Seek>(zin: &mut ZipArchive<R>) -> io::Result<Option<usize>> {
        for i in 0..zin.len() {
            let entry = zin.by_index(i)?;
            let name = entry.name().replace('\\', "/");
            if name == "mimetype" {
                return Ok(Some(i));
            }
        }
        Ok(None)
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
} // impl OfficeConverter

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
    #[cfg(windows)]
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
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
