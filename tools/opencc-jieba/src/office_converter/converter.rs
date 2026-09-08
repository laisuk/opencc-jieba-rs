//! # OfficeConverter Module
//!
//! Generic conversion support for text stored inside ZIP-based Office,
//! OpenDocument, and EPUB files.
//!
//! The document layer is deliberately independent of any OpenCC implementation.
//! Callers provide an [`OfficeTextConverter`], which wraps a text-conversion
//! function with the signature `Fn(&str, &str, bool) -> String`.
//!
//! This separation lets applications compose any conversion policy they need
//! (for example normalization, OpenCC conversion, custom dictionaries, or
//! DeToFu processing) without coupling this module to a particular converter.
//!
//! ## Supported formats
//!
//! - `.docx` (Word)
//! - `.xlsx` (Excel)
//! - `.pptx` (PowerPoint, including slides and notes)
//! - `.odt`, `.ods`, `.odp` (OpenDocument)
//! - `.epub`
//!
//! ## Features
//!
//! - Streams ZIP-based documents from an input path to a temporary output file.
//! - Supports in-memory ZIP conversion through [`OfficeConverter::convert_bytes`].
//! - Preserves non-target ZIP entries without recompression where possible.
//! - Optionally preserves font declarations by masking and restoring them.
//! - Converts XLSX worksheet text narrowly, leaving formulas and metadata untouched.
//! - Writes EPUB `mimetype` first and without compression, as required by EPUB.
//!
//! ## Example
//!
//! ```rust,no_run
//! use crate::converter::{OfficeConverter, OfficeTextConverter};
//!
//! let text_converter = OfficeTextConverter::new(
//!     |text: &str, _config: &str, _punctuation: bool| {
//!         text.replace("汉语", "漢語")
//!     },
//! );
//!
//! let result = OfficeConverter::convert(
//!     "input.docx",
//!     "output.docx",
//!     "docx",
//!     "s2t",
//!     true,
//!     true,
//!     &text_converter,
//! )?;
//!
//! assert!(result.success);
//! # Ok::<(), std::io::Error>(())
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

/// Result of a document conversion operation.
///
/// Holds a success flag and an explanatory message.
pub struct ConversionResult {
    pub success: bool,
    pub message: Box<str>,
}

/// Caller-supplied text conversion policy used by [`OfficeConverter`].
///
/// `OfficeTextConverter` is a lightweight adapter around any closure or function
/// implementing `Fn(&str, &str, bool) -> String`.
///
/// The three arguments are:
///
/// 1. the text fragment to convert,
/// 2. the requested conversion configuration (for example `"s2t"`),
/// 3. whether punctuation conversion is enabled.
///
/// The adapter intentionally knows nothing about OpenCC. A caller can therefore
/// wrap an OpenCC implementation, a test double, or a richer pipeline that adds
/// normalization or postprocessing.
///
/// # Example
///
/// ```rust
/// use crate::converter::OfficeTextConverter;
///
/// let converter = OfficeTextConverter::new(
///     |text: &str, config: &str, punctuation: bool| {
///         format!("{config}:{punctuation}:{text}")
///     },
/// );
///
/// assert_eq!(
///     converter.convert_text("漢字", "t2s", true),
///     "t2s:true:漢字"
/// );
/// ```
pub struct OfficeTextConverter<F> {
    convert: F,
}

impl<F> OfficeTextConverter<F>
where
    F: Fn(&str, &str, bool) -> String,
{
    /// Create a text-conversion adapter from a closure or function.
    #[inline]
    pub fn new(convert: F) -> Self {
        Self { convert }
    }

    /// Convert one text fragment using the wrapped conversion policy.
    #[inline]
    pub fn convert_text(&self, text: &str, config: &str, punctuation: bool) -> String {
        (self.convert)(text, config, punctuation)
    }
}

/// Generic converter for Office, OpenDocument, and EPUB archives.
///
/// `OfficeConverter` owns no conversion engine or dictionary. All text policy is
/// supplied through [`OfficeTextConverter`], keeping document parsing and ZIP
/// packaging independent of OpenCC or any other text-conversion library.
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
    /// Convert an Office/EPUB document from an input path to an output path.
    ///
    /// `text_converter` supplies the text conversion policy. The document layer
    /// does not depend on a particular OpenCC implementation.
    ///
    /// The output is written transactionally through a temporary file and is
    /// validated as a ZIP archive before replacing `output_path`.
    pub fn convert<F>(
        input_path: &str,
        output_path: &str,
        format: &str,
        config: &str,
        punctuation: bool,
        keep_font: bool,
        text_converter: &OfficeTextConverter<F>,
    ) -> io::Result<ConversionResult>
    where
        F: Fn(&str, &str, bool) -> String,
    {
        Self::convert_path_stream(
            input_path,
            output_path,
            format,
            config,
            punctuation,
            keep_font,
            text_converter,
        )
    }

    /// Convert a ZIP-based Office/EPUB document from in-memory bytes.
    ///
    /// Returns the converted ZIP bytes together with the number of target
    /// XML/XHTML entries that were processed.
    #[allow(dead_code)]
    pub fn convert_bytes<F>(
        input_zip: &[u8],
        format: &str,
        config: &str,
        punctuation: bool,
        keep_font: bool,
        text_converter: &OfficeTextConverter<F>,
    ) -> io::Result<(Vec<u8>, usize)>
    where
        F: Fn(&str, &str, bool) -> String,
    {
        Self::validate_input_zip(input_zip)?;
        let format = Self::normalize_format(format)?;
        let reader = Cursor::new(input_zip);

        let out_cursor = Cursor::new(Vec::<u8>::new());
        let mut z_out = ZipWriter::new(out_cursor);

        let converted_count = Self::convert_zip_stream(
            reader,
            &mut z_out,
            &format,
            config,
            punctuation,
            keep_font,
            text_converter,
        )?;

        let out_cursor = z_out.finish()?;
        let out_bytes = out_cursor.into_inner();
        Self::validate_zip_bytes(&out_bytes)?;
        Ok((out_bytes, converted_count))
    }

    /// Convert an Office/EPUB document by streaming from one filesystem path
    /// to another.
    ///
    /// Most callers should use [`Self::convert`]. This method is public for
    /// callers that specifically want the path-streaming entry point.
    pub fn convert_path_stream<F>(
        input_path: &str,
        output_path: &str,
        format: &str,
        config: &str,
        punctuation: bool,
        keep_font: bool,
        text_converter: &OfficeTextConverter<F>,
    ) -> io::Result<ConversionResult>
    where
        F: Fn(&str, &str, bool) -> String,
    {
        let format = Self::normalize_format(format)?;
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
                &format,
                config,
                punctuation,
                keep_font,
                text_converter,
            )?;

            Ok(())
        })?;

        Ok(ConversionResult {
            success: true,
            message: "✅ Conversion completed.".into(),
        })
    }

    /// Core ZIP-to-ZIP conversion engine.
    fn convert_zip_stream<R, W, F>(
        reader: R,
        z_out: &mut ZipWriter<W>,
        format: &str,
        config: &str,
        punctuation: bool,
        keep_font: bool,
        text_converter: &OfficeTextConverter<F>,
    ) -> io::Result<usize>
    where
        R: Read + Seek,
        W: Write + Seek,
        F: Fn(&str, &str, bool) -> String,
    {
        let mut zin = ZipArchive::new(reader)?;
        let mut converted_count = 0;

        // EPUB requires `mimetype` to be the first entry and stored uncompressed.
        let mut mimetype_index = None;
        if format.eq_ignore_ascii_case("epub") {
            mimetype_index = Self::find_mimetype_index(&mut zin)?;
            let Some(mi) = mimetype_index else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "EPUB is missing required mimetype entry",
                ));
            };

            let mut entry = zin.by_index(mi)?;
            let name = entry.name().replace('\\', "/");

            if !Self::is_unsafe_path(Path::new(&name)) && !entry.is_dir() && name == "mimetype" {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;

                let opts: FileOptions<'_, ExtendedFileOptions> =
                    FileOptions::default().compression_method(CompressionMethod::Stored);

                z_out.start_file("mimetype", opts)?;
                z_out.write_all(&buf)?;
            }
        }

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

                // For XLSX sharedStrings.xml font masking is safe. Worksheet XML
                // must use the narrow inline-string converter below so metadata and
                // formulas remain untouched.
                if keep_font && (!format.eq_ignore_ascii_case("xlsx") || is_xlsx_shared_strings) {
                    Self::mask_font(&mut content, format, &mut font_map);
                }

                let mut converted = if format.eq_ignore_ascii_case("xlsx") {
                    Self::convert_xlsx_entry(&content, &name, config, punctuation, text_converter)
                } else {
                    text_converter.convert_text(&content, config, punctuation)
                };

                for (marker, original) in font_map {
                    converted = converted.replace(&marker, &original);
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

        if converted_count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("No valid XML/XHTML fragments were converted for format '{format}'."),
            ));
        }

        Ok(converted_count)
    }

    fn validate_input_zip(input_zip: &[u8]) -> io::Result<()> {
        if input_zip.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "input ZIP bytes must not be empty",
            ));
        }
        Ok(())
    }

    fn normalize_format(format: &str) -> io::Result<String> {
        let normalized = format.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "format must not be empty",
            ));
        }

        if !Self::is_supported_format(&normalized) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Unsupported Office/EPUB format: '{format}'."),
            ));
        }

        Ok(normalized)
    }

    fn is_supported_format(format: &str) -> bool {
        matches!(
            format,
            "docx" | "xlsx" | "pptx" | "odt" | "ods" | "odp" | "epub"
        )
    }

    fn validate_zip_bytes(bytes: &[u8]) -> io::Result<()> {
        let cursor = Cursor::new(bytes);
        let _ = ZipArchive::new(cursor)?;
        Ok(())
    }

    fn validate_zip_file(path: &Path) -> io::Result<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let _ = ZipArchive::new(reader)?;
        Ok(())
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
    fn convert_xlsx_entry<F>(
        content: &str,
        name: &str,
        config: &str,
        punctuation: bool,
        text_converter: &OfficeTextConverter<F>,
    ) -> String
    where
        F: Fn(&str, &str, bool) -> String,
    {
        if Self::is_xlsx_shared_strings(name) {
            return text_converter.convert_text(content, config, punctuation);
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

                                let converted =
                                    text_converter.convert_text(inner_text, config, punctuation);
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

    OfficeConverter::validate_zip_file(&tmp_out)?;

    remove_existing_file(final_out)?;
    fs::rename(&tmp_out, final_out)?;

    guard.commit();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use zip::{
        write::{ExtendedFileOptions, FileOptions},
        CompressionMethod, ZipArchive, ZipWriter,
    };

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut input_cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut zip = ZipWriter::new(&mut input_cursor);
            let opts: FileOptions<'_, ExtendedFileOptions> =
                FileOptions::default().compression_method(CompressionMethod::Deflated);

            for (name, bytes) in entries {
                zip.start_file(*name, opts.clone()).unwrap();
                zip.write_all(bytes).unwrap();
            }

            zip.finish().unwrap();
        }
        input_cursor.into_inner()
    }

    #[test]
    fn test_convert_bytes_rejects_empty_input() {
        let converter = OfficeTextConverter::new(|text: &str, _, _| text.to_owned());
        let err = OfficeConverter::convert_bytes(&[], "docx", "s2t", true, true, &converter)
            .expect_err("empty input must be rejected");

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn test_convert_bytes_rejects_invalid_zip() {
        let converter = OfficeTextConverter::new(|text: &str, _, _| text.to_owned());
        let err =
            OfficeConverter::convert_bytes(b"not a zip", "docx", "s2t", true, true, &converter)
                .expect_err("invalid ZIP input must be rejected");

        assert_ne!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_convert_bytes_rejects_unsupported_format() {
        let converter = OfficeTextConverter::new(|text: &str, _, _| text.to_owned());
        let zip = make_zip(&[(
            "word/document.xml",
            "<w:document>汉语</w:document>".as_bytes(),
        )]);
        let err = OfficeConverter::convert_bytes(&zip, "pdf", "s2t", true, true, &converter)
            .expect_err("unsupported format must be rejected");

        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("Unsupported Office/EPUB format"));
    }

    #[test]
    fn test_convert_bytes_rejects_zip_with_no_target_fragments() {
        let converter = OfficeTextConverter::new(|text: &str, _, _| text.to_owned());
        let zip = make_zip(&[("docProps/core.xml", "<root>汉语</root>".as_bytes())]);
        let err = OfficeConverter::convert_bytes(&zip, "docx", "s2t", true, true, &converter)
            .expect_err("ZIP without target XML must be rejected");

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("No valid XML/XHTML fragments"));
    }

    #[test]
    fn test_convert_bytes_rejects_epub_without_mimetype() {
        let converter = OfficeTextConverter::new(|text: &str, _, _| text.to_owned());
        let zip = make_zip(&[(
            "OEBPS/content.xhtml",
            "<html><body>汉语</body></html>".as_bytes(),
        )]);
        let err = OfficeConverter::convert_bytes(&zip, "epub", "s2t", true, true, &converter)
            .expect_err("EPUB without mimetype must be rejected");

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("mimetype"));
    }
    #[test]
    fn test_convert_bytes_xlsx_inline_string_cells() {
        let mut input_cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut zip = ZipWriter::new(&mut input_cursor);
            let opts: FileOptions<'_, ExtendedFileOptions> =
                FileOptions::default().compression_method(CompressionMethod::Deflated);

            zip.start_file("[Content_Types].xml", opts.clone()).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#)
                .unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            zip.write_all("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>汉语</t></is></c></row></sheetData></worksheet>".as_bytes())
                .unwrap();

            zip.finish().unwrap();
        }

        let converter = OfficeTextConverter::new(|text: &str, _, _| text.replace("汉语", "漢語"));

        let (out_bytes, converted_count) = OfficeConverter::convert_bytes(
            input_cursor.get_ref(),
            "xlsx",
            "s2t",
            true,
            true,
            &converter,
        )
        .expect("convert_bytes failed");

        assert_eq!(
            converted_count, 1,
            "Expected the worksheet inline-string XML to be converted"
        );

        let cursor = Cursor::new(out_bytes);
        let mut zip = ZipArchive::new(cursor).expect("Output is not a valid ZIP archive");
        let mut sheet = zip
            .by_name("xl/worksheets/sheet1.xml")
            .expect("Converted xlsx is missing xl/worksheets/sheet1.xml");
        let mut content = String::new();
        sheet.read_to_string(&mut content).unwrap();

        assert!(
            content.contains("漢語"),
            "Expected inline string content to be converted, got: {content}"
        );
    }

    #[test]
    fn test_convert_bytes_xlsx_formula_untouched() {
        let mut input_cursor = Cursor::new(Vec::<u8>::new());
        {
            let mut zip = ZipWriter::new(&mut input_cursor);
            let opts: FileOptions<'_, ExtendedFileOptions> =
                FileOptions::default().compression_method(CompressionMethod::Deflated);

            zip.start_file("[Content_Types].xml", opts.clone()).unwrap();
            zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"></Types>"#)
                .unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            zip.write_all(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
                 <worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
                 <sheetData><row r=\"1\">\
                 <c r=\"A1\" t=\"inlineStr\"><is><t>汉语</t></is></c>\
                 <c r=\"B1\"><f>CONCAT(\"汉语\", \"A\")</f></c>\
                 </row></sheetData></worksheet>"
                    .as_bytes(),
            )
            .unwrap();

            zip.finish().unwrap();
        }

        let converter = OfficeTextConverter::new(|text: &str, _, _| text.replace("汉语", "漢語"));

        let (out_bytes, _) = OfficeConverter::convert_bytes(
            input_cursor.get_ref(),
            "xlsx",
            "s2t",
            true,
            true,
            &converter,
        )
        .expect("convert_bytes failed");

        let cursor = Cursor::new(out_bytes);
        let mut zip = ZipArchive::new(cursor).expect("Output is not a valid ZIP archive");
        let mut sheet = zip
            .by_name("xl/worksheets/sheet1.xml")
            .expect("Converted xlsx is missing xl/worksheets/sheet1.xml");
        let mut content = String::new();
        sheet.read_to_string(&mut content).unwrap();

        assert!(content.contains("漢語"));
        assert!(content.contains(r#"<f>CONCAT("汉语", "A")</f>"#));
    }
    #[test]
    fn test_convert_bytes_uses_office_text_converter() {
        let zip = make_zip(&[(
            "word/document.xml",
            "<w:document>汉语</w:document>".as_bytes(),
        )]);

        let converter =
            OfficeTextConverter::new(|text: &str, _config: &str, _punctuation: bool| {
                text.replace("汉语", "自訂")
            });

        let (out_bytes, converted_count) =
            OfficeConverter::convert_bytes(&zip, "docx", "s2t", false, false, &converter)
                .expect("delegate conversion failed");

        assert_eq!(converted_count, 1);

        let cursor = Cursor::new(out_bytes);
        let mut zip = ZipArchive::new(cursor).expect("Output is not a valid ZIP archive");
        let mut document = zip
            .by_name("word/document.xml")
            .expect("Converted docx is missing word/document.xml");
        let mut content = String::new();
        document.read_to_string(&mut content).unwrap();

        assert!(content.contains("自訂"));
    }

    #[test]
    fn test_convert_bytes_docx_with_extended_compat_normalization() {
        use opencc_jieba_rs::OpenCC;
        let input_text = "天龍八部書裡的喬峰是契丹人‧聼聼竒羙⽟䂖甁噐⾳";

        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r>
        <w:t>{input_text}</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#
        );

        let zip = make_zip(&[("word/document.xml", xml.as_bytes())]);

        let helper = OpenCC::new();

        let text_converter =
            OfficeTextConverter::new(|text: &str, config: &str, punctuation: bool| {
                let normalized = helper.normalize_compat_extended(text);
                helper.convert(&normalized, config, punctuation)
            });

        let (out_bytes, converted_count) =
            OfficeConverter::convert_bytes(&zip, "docx", "t2s", true, false, &text_converter)
                .expect("DOCX extended compatibility conversion failed");

        assert_eq!(converted_count, 1);

        let cursor = Cursor::new(out_bytes);
        let mut zip = ZipArchive::new(cursor).expect("Output is not a valid ZIP archive");

        let mut document = zip
            .by_name("word/document.xml")
            .expect("Converted DOCX is missing word/document.xml");

        let mut content = String::new();
        document.read_to_string(&mut content).unwrap();

        assert!(
            content.contains("天龙八部书里的乔峰是契丹人·听听奇美玉石瓶器音"),
            "Unexpected converted DOCX content: {content}"
        );
    }
}
