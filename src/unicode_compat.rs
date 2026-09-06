//! Curated Unicode compatibility normalization for Chinese text.
//!
//! This module provides an optional Unicode compatibility pre-processing pass
//! for Chinese text, especially text originating from PDF extraction and other
//! glyph-oriented sources.
//!
//! The built-in extended table is loaded from
//! `data/Unicode_Compatibility.txt` with [`include_bytes!`], parsed once, and
//! cached with [`std::sync::OnceLock`]. The table contains curated one-scalar to
//! one-scalar mappings such as Unicode radicals, glyph variants, punctuation
//! forms, and known text-extraction artifacts.
//!
//! This is intentionally **not** a general-purpose Unicode normalization
//! implementation such as NFC, NFD, NFKC, or NFKD.
//!
//! # Two normalization levels
//!
//! [`UnicodeCompat::normalize`] applies only the curated mappings from
//! `Unicode_Compatibility.txt`.
//!
//! [`UnicodeCompat::normalize_all`] first gives the built-in
//! [`CompatIdeographs`] table precedence for CJK Compatibility Ideographs, then
//! falls back to the curated extended table. This is the intended implementation
//! behind higher-level "extended compatibility" normalization.
//!
//! # Invariants
//!
//! Every mapping is exactly one Unicode scalar value to one Unicode scalar
//! value. Multi-scalar sources and targets are rejected when the table is
//! parsed. This keeps normalization position-stable at the Unicode-scalar level.
//!
//! ASCII source mappings are also rejected so that the extended table cannot
//! accidentally rewrite ASCII markup or structured-text syntax.

use crate::compat_ideographs::CompatIdeographs;
use rustc_hash::FxHashMap;
use std::sync::OnceLock;

static UNICODE_COMPAT_DATA: &[u8] = include_bytes!("data/Unicode_Compatibility.txt");
static UNICODE_COMPAT_TABLE: OnceLock<UnicodeCompat> = OnceLock::new();

/// Curated Unicode compatibility normalizer.
///
/// `UnicodeCompat` combines two independent normalization sources:
///
/// - the existing [`CompatIdeographs`] table for Unicode CJK Compatibility
///   Ideographs; and
/// - a sparse curated table loaded from `data/Unicode_Compatibility.txt`.
///
/// The curated table is stored in an [`FxHashMap`] because its source characters
/// are sparse and are not confined to one compact Unicode range.
///
/// The built-in instance is immutable after initialization and can be shared
/// safely across threads.
#[derive(Debug, Clone)]
pub(crate) struct UnicodeCompat {
    compat: &'static CompatIdeographs,
    extended: FxHashMap<char, char>,
}

impl UnicodeCompat {
    /// Returns the cached built-in Unicode compatibility normalizer.
    ///
    /// `data/Unicode_Compatibility.txt` is embedded in the crate binary with
    /// [`include_bytes!`] and parsed at most once per process. Subsequent calls
    /// reuse the same immutable table.
    ///
    /// # Panics
    ///
    /// Panics if the bundled table is not valid UTF-8 or violates the mapping
    /// format documented by [`UnicodeCompat::from_text`]. Such a failure
    /// indicates invalid crate data rather than invalid user input.
    pub(crate) fn builtin() -> &'static Self {
        UNICODE_COMPAT_TABLE.get_or_init(|| {
            let text = std::str::from_utf8(UNICODE_COMPAT_DATA)
                .expect("Unicode_Compatibility.txt must be valid UTF-8");

            Self::from_text(text)
                .unwrap_or_else(|err| panic!("invalid Unicode_Compatibility.txt: {err}"))
        })
    }

    /// Builds a Unicode compatibility normalizer from mapping text.
    ///
    /// This constructor is mainly useful for tests, generated data, and advanced
    /// callers that need to validate a table before using it.
    ///
    /// # Format
    ///
    /// Each non-comment line must contain exactly two tab-separated columns:
    ///
    /// ```text
    /// source<TAB>target
    /// ```
    ///
    /// Both `source` and `target` must contain exactly one Unicode scalar value.
    /// Blank lines and lines beginning with `#` are ignored.
    ///
    /// ASCII source characters (`U+0000..=U+007F`) are rejected. This prevents
    /// the compatibility table from accidentally rewriting ASCII markup,
    /// OpenXML/XML syntax, paths, command text, or other structured content.
    ///
    /// Duplicate source entries use **last-wins** semantics, matching the stable
    /// OpenccNet `UnicodeCompat` implementation.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error containing the source line number when a row:
    ///
    /// - is missing a target;
    /// - contains more than two tab-separated columns;
    /// - has an empty source or target;
    /// - has a source or target containing more than one Unicode scalar; or
    /// - uses an ASCII source character.
    ///
    ///
    pub(crate) fn from_text(text: &str) -> Result<Self, String> {
        let mut extended = FxHashMap::default();

        for (index, raw_line) in text.lines().enumerate() {
            let line_no = index + 1;

            if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
                continue;
            }

            let mut parts = raw_line.split('\t');

            let src_text = parts
                .next()
                .map(str::trim)
                .ok_or_else(|| format!("line {line_no}: missing source"))?;

            let dst_text = parts
                .next()
                .map(str::trim)
                .ok_or_else(|| format!("line {line_no}: missing target"))?;

            if parts.next().is_some() {
                return Err(format!("line {line_no}: too many columns"));
            }

            let src = single_char(src_text, line_no, "source")?;
            if src.is_ascii() {
                return Err(format!(
                    "line {line_no}: source must not be an ASCII character"
                ));
            }

            let dst = single_char(dst_text, line_no, "target")?;

            // Deliberately last-wins, matching the stable C# implementation.
            extended.insert(src, dst);
        }

        Ok(Self {
            compat: CompatIdeographs::builtin(),
            extended,
        })
    }

    /// Normalizes one character using only the curated extended table.
    ///
    /// CJK Compatibility Ideograph normalization is **not** applied by this
    /// method. Use [`normalize_all_char`](Self::normalize_all_char) when both
    /// tables should participate.
    ///
    /// Characters without an extended mapping are returned unchanged.
    ///
    ///
    #[inline(always)]
    pub(crate) fn normalize_char(&self, ch: char) -> char {
        if ch.is_ascii() {
            return ch;
        }

        self.extended.get(&ch).copied().unwrap_or(ch)
    }

    /// Normalizes one character using CJK Compatibility Ideographs first, then
    /// the curated extended table.
    ///
    /// The existing [`CompatIdeographs`] mapping has precedence. If it changes
    /// the input character, that result is returned directly and is **not** fed
    /// through the extended table a second time. Otherwise, the curated table is
    /// consulted.
    ///
    /// This precedence matches the stable OpenccNet `UnicodeCompat.NormalizeAll`
    /// behavior and avoids accidental chained remapping between the two tables.
    ///
    ///
    #[inline(always)]
    pub(crate) fn normalize_all_char(&self, ch: char) -> char {
        if ch.is_ascii() {
            return ch;
        }

        let compat = self.compat.normalize_char(ch);
        if compat != ch {
            return compat;
        }

        self.extended.get(&ch).copied().unwrap_or(ch)
    }

    /// Normalizes text using only the curated mappings from
    /// `Unicode_Compatibility.txt`.
    ///
    /// This method does not apply [`CompatIdeographs`]. It allocates one output
    /// [`String`] and preserves every unmapped character unchanged.
    ///
    /// Because every mapping is one Unicode scalar to one Unicode scalar, the
    /// number of Unicode scalar values in the output is identical to the input,
    /// although the UTF-8 byte length may differ.
    ///
    ///
    pub(crate) fn normalize(&self, input: &str) -> String {
        self.normalize_impl(input, false)
    }

    /// Normalizes text using both the built-in CJK Compatibility Ideograph
    /// mappings and the curated extended table.
    ///
    /// For each Unicode scalar value, [`CompatIdeographs`] is consulted first.
    /// Only when it leaves the character unchanged is the curated extended map
    /// consulted. The result of one table is never passed through the other table
    /// again.
    ///
    /// This is the intended low-level implementation for a higher-level
    /// `normalize_compat_extended()` API.
    ///
    ///
    pub(crate) fn normalize_all(&self, input: &str) -> String {
        self.normalize_impl(input, true)
    }

    fn normalize_impl(&self, input: &str, include_compat: bool) -> String {
        let mut output = String::with_capacity(input.len());

        for ch in input.chars() {
            output.push(if include_compat {
                self.normalize_all_char(ch)
            } else {
                self.normalize_char(ch)
            });
        }

        output
    }

    /// Normalizes a mutable character slice using only the curated extended
    /// table.
    ///
    /// This is useful when callers already own a reusable `Vec<char>` before
    /// segmentation or conversion.
    ///
    ///
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn normalize_in_place(&self, chars: &mut [char]) {
        for ch in chars {
            *ch = self.normalize_char(*ch);
        }
    }

    /// Normalizes a mutable character slice using both compatibility tables.
    ///
    /// [`CompatIdeographs`] has precedence over the curated extended table for
    /// each character, exactly as in [`normalize_all`](Self::normalize_all).
    ///
    ///
    #[cfg(test)]
    pub(crate) fn normalize_all_in_place(&self, chars: &mut [char]) {
        for ch in chars {
            *ch = self.normalize_all_char(*ch);
        }
    }
}

fn single_char(text: &str, line_no: usize, field: &str) -> Result<char, String> {
    let mut chars = text.chars();

    let ch = chars
        .next()
        .ok_or_else(|| format!("line {line_no}: empty {field}"))?;

    if chars.next().is_some() {
        return Err(format!(
            "line {line_no}: {field} must be exactly one Unicode scalar value"
        ));
    }

    Ok(ch)
}

/// Normalizes text using only the built-in curated
/// `Unicode_Compatibility.txt` table.
///
/// This convenience wrapper does **not** apply CJK Compatibility Ideograph
/// normalization. Use [`normalize_unicode_compat_all`] when both tables are
/// desired.
///
///
pub(crate) fn normalize_unicode_compat(input: &str) -> String {
    UnicodeCompat::builtin().normalize(input)
}

/// Normalizes text using both the built-in CJK Compatibility Ideograph table
/// and the curated `Unicode_Compatibility.txt` table.
///
/// CJK Compatibility Ideograph mappings have precedence for each input
/// character. The extended table is consulted only when the compatibility
/// ideograph table leaves that character unchanged.
///
/// This function is useful as the implementation behind a higher-level
/// `OpenCC::normalize_compat_extended()` method.
///
///
pub(crate) fn normalize_unicode_compat_all(input: &str) -> String {
    UnicodeCompat::builtin().normalize_all(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comments_blank_lines_and_pairs() {
        let table = UnicodeCompat::from_text(
            "\
# comment

⺙\t攵
聼\t聽
",
        )
        .unwrap();

        assert_eq!(table.normalize("⺙聼"), "攵聽");
    }

    #[test]
    fn extended_normalization_does_not_apply_compat_ideographs() {
        let table = UnicodeCompat::from_text("⺙\t攵\n").unwrap();

        assert_eq!(table.normalize("金⺙"), "金攵");
    }

    #[test]
    fn normalize_all_combines_compat_and_extended_tables() {
        let table = UnicodeCompat::from_text("⺙\t攵\n").unwrap();

        assert_eq!(table.normalize_all("金⺙"), "金攵");
    }

    #[test]
    fn compat_mapping_has_precedence_without_chained_remapping() {
        // 金 is normalized by CompatIdeographs to 金. If normalize_all were
        // implemented as two chained full passes, the extended 金 -> 銀 entry
        // would incorrectly turn the result into 銀.
        let table = UnicodeCompat::from_text("金\t銀\n").unwrap();

        assert_eq!(table.normalize("金金"), "金銀");
        assert_eq!(table.normalize_all("金金"), "金銀");
    }

    #[test]
    fn duplicate_sources_are_last_wins() {
        let table = UnicodeCompat::from_text(
            "\
聼\t听
聼\t聽
",
        )
        .unwrap();

        assert_eq!(table.normalize("聼"), "聽");
    }

    #[test]
    fn rejects_ascii_source() {
        let err = UnicodeCompat::from_text("A\tＢ\n").unwrap_err();

        assert_eq!(err, "line 1: source must not be an ASCII character");
    }

    #[test]
    fn rejects_missing_target() {
        let err = UnicodeCompat::from_text("聼\n").unwrap_err();

        assert_eq!(err, "line 1: missing target");
    }

    #[test]
    fn rejects_too_many_columns() {
        let err = UnicodeCompat::from_text("聼\t聽\textra\n").unwrap_err();

        assert_eq!(err, "line 1: too many columns");
    }

    #[test]
    fn rejects_empty_source_or_target() {
        assert_eq!(
            UnicodeCompat::from_text("\t聽\n").unwrap_err(),
            "line 1: empty source"
        );
        assert_eq!(
            UnicodeCompat::from_text("聼\t\n").unwrap_err(),
            "line 1: empty target"
        );
    }

    #[test]
    fn rejects_multi_scalar_source_or_target() {
        assert_eq!(
            UnicodeCompat::from_text("聼x\t聽\n").unwrap_err(),
            "line 1: source must be exactly one Unicode scalar value"
        );
        assert_eq!(
            UnicodeCompat::from_text("聼\t聽x\n").unwrap_err(),
            "line 1: target must be exactly one Unicode scalar value"
        );
    }

    #[test]
    fn supports_astral_source_and_target_scalars() {
        let table = UnicodeCompat::from_text("𠮷\t𠮟\n").unwrap();

        assert_eq!(table.normalize("A𠮷B"), "A𠮟B");
    }

    #[test]
    fn ascii_and_unmapped_text_stay_unchanged() {
        let table = UnicodeCompat::from_text("聼\t聽\n").unwrap();

        assert_eq!(table.normalize("ABC123 中文"), "ABC123 中文");
        assert_eq!(table.normalize_all("ABC123 中文"), "ABC123 中文");
    }

    #[test]
    fn normalize_in_place_uses_extended_table_only() {
        let table = UnicodeCompat::from_text("⺙\t攵\n").unwrap();
        let mut chars: Vec<char> = "金⺙".chars().collect();

        table.normalize_in_place(&mut chars);

        assert_eq!(chars.into_iter().collect::<String>(), "金攵");
    }

    #[test]
    fn normalize_all_in_place_combines_both_tables() {
        let table = UnicodeCompat::from_text("⺙\t攵\n").unwrap();
        let mut chars: Vec<char> = "金⺙".chars().collect();

        table.normalize_all_in_place(&mut chars);

        assert_eq!(chars.into_iter().collect::<String>(), "金攵");
    }

    #[test]
    fn empty_table_is_valid() {
        let table = UnicodeCompat::from_text("# comments only\n\n").unwrap();

        assert_eq!(table.normalize("聼"), "聼");
        assert_eq!(table.normalize_all("金"), "金");
    }

    #[test]
    fn builtin_is_cached() {
        let a = UnicodeCompat::builtin();
        let b = UnicodeCompat::builtin();

        assert!(std::ptr::eq(a, b));
    }
}
