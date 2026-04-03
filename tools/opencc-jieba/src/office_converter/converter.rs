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

use regex::{Captures, Regex};
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

/// Precompiled regex patterns for XLSX inline-string handling.
struct XlsxPatterns {
    any_cell: Regex,
    text_node: Regex,
}

impl XlsxPatterns {
    fn new() -> Self {
        Self {
            any_cell: Regex::new(r#"<c\b[^>]*>.*?</c>"#).unwrap(),
            text_node: Regex::new(r#"(<t\b[^>]*>)(.*?)(</t>)"#).unwrap(),
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

    /// Thread-local storage for XLSX inline-string regex patterns.
    static XLSX_PATTERNS: XlsxPatterns = XlsxPatterns::new();
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

    /// Convert a ZIP-based Office / EPUB document from in-memory bytes and return the output bytes.
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

        let out_cursor = z_out.finish()?;
        Ok((out_cursor.into_inner(), converted_count))
    }

    /// Convert an Office / EPUB document from an input file path to an output file path
    /// using the streaming ZIP conversion core.
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
            if format.eq_ignore_ascii_case("epub") && mimetype_index == Some(i) {
                continue;
            }

            let mut entry = zin.by_index(i)?;
            let name = entry.name().replace('\\', "/");

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

                let is_xlsx_shared_strings =
                    format.eq_ignore_ascii_case("xlsx") && Self::is_xlsx_shared_strings(&name);

                let is_xlsx_worksheet =
                    format.eq_ignore_ascii_case("xlsx") && Self::is_xlsx_worksheet(&name);

                // For XLSX:
                // - sharedStrings.xml: font masking may still be applied if requested
                // - worksheet XML: do NOT broad-mask val="..." metadata; narrow conversion only
                if keep_font && (!format.eq_ignore_ascii_case("xlsx") || is_xlsx_shared_strings) {
                    Self::mask_font(&mut content, format, &mut font_map);
                }

                let mut converted = if format.eq_ignore_ascii_case("xlsx") {
                    Self::convert_xlsx_entry(&content, &name, helper, config, punctuation)
                } else {
                    helper.convert(&content, config, punctuation)
                };

                if !font_map.is_empty() {
                    for (marker, original) in font_map {
                        converted = converted.replace(&marker, &original);
                    }
                }

                let opts: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(CompressionMethod::Deflated);

                z_out.start_file(name, opts)?;
                z_out.write_all(converted.as_bytes())?;
                converted_count += 1;

                let _ = is_xlsx_worksheet; // keep explicit logic readable without warnings if reshaped later
            } else {
                z_out.raw_copy_file(entry)?;
            }
        }

        Ok(converted_count)
    }

    /// Determine if a ZIP entry name should be converted for the given format.
    fn is_target_entry(format: &str, name: &str) -> bool {
        match format {
            "docx" => name == "word/document.xml",
            "xlsx" => {
                name == "xl/sharedStrings.xml"
                    || (name.starts_with("xl/worksheets/") && name.ends_with(".xml"))
            }
            "pptx" => {
                let is_xml = name.ends_with(".xml");
                let is_rels = name.ends_with(".rels");
                let in_slides = name.starts_with("ppt/slides/");
                let in_notes = name.starts_with("ppt/notesSlides/");
                is_xml && !is_rels && (in_slides || in_notes)
            }
            "odt" | "ods" | "odp" => name == "content.xml",
            "epub" => {
                let lower = name.to_ascii_lowercase();
                lower.ends_with(".xhtml")
                    || lower.ends_with(".opf")
                    || lower.ends_with(".ncx")
                    || lower.ends_with(".html")
            }
            _ => false,
        }
    }

    #[inline]
    fn is_xlsx_shared_strings(name: &str) -> bool {
        name == "xl/sharedStrings.xml"
    }

    #[inline]
    fn is_xlsx_worksheet(name: &str) -> bool {
        name.starts_with("xl/worksheets/") && name.ends_with(".xml")
    }

    /// Convert a single XLSX entry using narrow rules:
    /// - sharedStrings.xml => whole-file conversion
    /// - worksheet XML => only inline-string cell text nodes
    /// - other XML => unchanged
    fn convert_xlsx_entry(
        content: &str,
        name: &str,
        helper: &OpenCC,
        config: &str,
        punctuation: bool,
    ) -> String {
        if Self::is_xlsx_shared_strings(name) {
            return helper.convert(content, config, punctuation);
        }

        if Self::is_xlsx_worksheet(name) {
            return XLSX_PATTERNS.with(|patterns| {
                patterns
                    .any_cell
                    .replace_all(content, |cell_caps: &Captures| {
                        let cell_xml = cell_caps.get(0).map(|m| m.as_str()).unwrap_or_default();

                        if !Self::is_inline_string_cell(cell_xml) {
                            return cell_xml.to_owned();
                        }

                        patterns
                            .text_node
                            .replace_all(cell_xml, |text_caps: &Captures| {
                                let open_tag =
                                    text_caps.get(1).map(|m| m.as_str()).unwrap_or_default();
                                let inner_text =
                                    text_caps.get(2).map(|m| m.as_str()).unwrap_or_default();
                                let close_tag =
                                    text_caps.get(3).map(|m| m.as_str()).unwrap_or_default();

                                if inner_text.is_empty() {
                                    return text_caps
                                        .get(0)
                                        .map(|m| m.as_str().to_owned())
                                        .unwrap_or_default();
                                }

                                let converted = helper.convert(inner_text, config, punctuation);
                                let mut out = String::with_capacity(
                                    open_tag.len() + converted.len() + close_tag.len(),
                                );
                                out.push_str(open_tag);
                                out.push_str(&converted);
                                out.push_str(close_tag);
                                out
                            })
                            .into_owned()
                    })
                    .into_owned()
            });
        }

        content.to_owned()
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
                let mut result_str = String::with_capacity(xml.len() + xml.len() / 10);
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

    #[inline]
    fn is_inline_string_cell(cell_xml: &str) -> bool {
        let Some(tag_end) = cell_xml.find('>') else {
            return false;
        };

        let open_tag = &cell_xml[..tag_end];
        open_tag.contains(r#"t="inlineStr""#) || open_tag.contains("t='inlineStr'")
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

    #[cfg(windows)]
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            fs::set_permissions(path, perms)?;
        }
    }

    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Write to a temp file then atomically replace the final path.
///
/// Ensures no partial/corrupted output if interrupted.
///
/// On failure, the temp file is removed best-effort so stale
/// `*.tmp.<ext>` files do not accumulate.
fn replace_with_temp(
    final_out: &Path,
    write_zip: impl FnOnce(&mut ZipWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    struct TempFileGuard {
        path: PathBuf,
        committed: bool,
    }

    impl TempFileGuard {
        #[inline]
        fn new(path: PathBuf) -> Self {
            Self {
                path,
                committed: false,
            }
        }

        #[inline]
        fn commit(&mut self) {
            self.committed = true;
        }
    }

    impl Drop for TempFileGuard {
        fn drop(&mut self) {
            if !self.committed {
                let _ = fs::remove_file(&self.path);
            }
        }
    }

    let ext = final_out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("zip");
    let tmp_out = final_out.with_extension(format!("tmp.{}", ext));

    let _ = remove_existing_file(&tmp_out);

    let mut guard = TempFileGuard::new(tmp_out.clone());

    {
        let zip_file = File::create(&tmp_out)?;
        let mut zw = ZipWriter::new(zip_file);
        write_zip(&mut zw)?;
        zw.finish()?;
    }

    remove_existing_file(final_out)?;
    fs::rename(&tmp_out, final_out)?;

    guard.commit();

    Ok(())
}
