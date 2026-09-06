//! Display compatibility fallback utilities.
//!
//! This module provides optional DeTofu processing for non-BMP CJK extension
//! characters that may not render correctly on some systems, fonts, browsers,
//! or e-book readers.
//!
//! The built-in fallback table covers rare characters that may be produced by
//! both Simplified-to-Traditional and Traditional-to-Simplified conversion.
//! DeTofu is direction-independent and is typically applied after OpenCC
//! conversion when the target renderer has incomplete rare-character coverage.
//!
//! The built-in fallback table is parsed and hashed lazily on first use, then
//! shared immutably by [`OpenCC`](crate::OpenCC) and [`DetofuMap`] instances.
//! A customizable [`DetofuMap`] stores only application-specific overrides, so
//! creating one does not clone the built-in table.

use rustc_hash::FxHashMap;
use std::path::Path;
use std::sync::OnceLock;

static TOFU_DATA: &[u8] = include_bytes!("data/CharactersTofu.txt");

/// Controls which CJK extension ranges are replaced by DeTofu.
///
/// DeTofu levels are threshold-based: the selected level is the earliest
/// extension block to replace, and all supported later extension blocks are
/// replaced too.
///
/// - [`DetofuLevel::ExtB`] means ExtB+ and replaces all supported non-BMP
///   mappings: ExtB, ExtC, ExtD, ExtE, ExtF, ExtG, ExtH, and ExtI.
/// - [`DetofuLevel::ExtC`] means ExtC+ and replaces ExtC through ExtI.
/// - [`DetofuLevel::ExtD`] means ExtD+ and replaces ExtD through ExtI.
/// - [`DetofuLevel::ExtE`] means ExtE+ and replaces ExtE through ExtI.
///
/// The CLI alias `all` maps to [`DetofuLevel::ExtB`], so `ExtB` is the
/// broadest built-in fallback level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetofuLevel {
    /// Replace CJK Extension B and all supported later extension mappings.
    ExtB,
    /// Replace CJK Extension C and all supported later extension mappings.
    ExtC,
    /// Replace CJK Extension D and all supported later extension mappings.
    ExtD,
    /// Replace CJK Extension E and all supported later extension mappings.
    ExtE,
    /// Replace CJK Extension F and all supported later extension mappings.
    ExtF,
    /// Replace CJK Extension G and all supported later extension mappings.
    ExtG,
    /// Replace CJK Extension H and all supported later extension mappings.
    ExtH,
    /// Replace CJK Extension I mappings.
    ExtI,
}

impl DetofuLevel {
    /// Parses a DeTofu threshold level.
    ///
    /// Parsing is ASCII case-insensitive and ignores surrounding whitespace.
    /// Each level accepts its compact letter (for example `"b"`), compact
    /// extension name (`"extb"`), and hyphenated extension name (`"ext-b"`).
    /// The `"all"` alias selects [`DetofuLevel::ExtB`].
    ///
    /// Returns a message listing the supported values when parsing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::DetofuLevel;
    ///
    /// assert_eq!(DetofuLevel::parse(" ext-c "), Ok(DetofuLevel::ExtC));
    /// assert!(DetofuLevel::parse("unsupported").is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "all" | "ext-b" | "extb" | "b" => Ok(Self::ExtB),
            "ext-c" | "extc" | "c" => Ok(Self::ExtC),
            "ext-d" | "extd" | "d" => Ok(Self::ExtD),
            "ext-e" | "exte" | "e" => Ok(Self::ExtE),
            "ext-f" | "extf" | "f" => Ok(Self::ExtF),
            "ext-g" | "extg" | "g" => Ok(Self::ExtG),
            "ext-h" | "exth" | "h" => Ok(Self::ExtH),
            "ext-i" | "exti" | "i" => Ok(Self::ExtI),
            _ => Err(
                "supported DeTofu levels: all, ext-b, ext-c, ext-d, ext-e, ext-f, ext-g, ext-h, ext-i"
                    .to_string(),
            ),
        }
    }
}

/// Shared built-in lookup table:
///
/// `source character -> (fallback character, source extension level)`
///
/// The table is initialized once and then remains immutable.
static TOFU_MAP: OnceLock<FxHashMap<char, (char, DetofuLevel)>> = OnceLock::new();

fn parse_tofu_entries(text: &str) -> Result<Vec<(char, char, DetofuLevel)>, String> {
    let mut entries = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split('\t');

        let tofu = parts
            .next()
            .and_then(|s| s.trim().chars().next())
            .ok_or_else(|| format!("line {line_no}: missing tofu character"))?;

        let fallback = parts
            .next()
            .and_then(|s| s.trim().chars().next())
            .ok_or_else(|| format!("line {line_no}: missing fallback character"))?;

        let ext_text = parts
            .next()
            .map(str::trim)
            .ok_or_else(|| format!("line {line_no}: missing extension"))?;

        let ext = DetofuLevel::parse(ext_text)
            .map_err(|err| format!("line {line_no}: invalid extension `{ext_text}`: {err}"))?;

        entries.push((tofu, fallback, ext));
    }

    Ok(entries)
}

fn tofu_map() -> &'static FxHashMap<char, (char, DetofuLevel)> {
    TOFU_MAP.get_or_init(|| {
        let text = std::str::from_utf8(TOFU_DATA).expect("CharactersTofu.txt must be valid UTF-8");

        parse_tofu_entries(text)
            .unwrap_or_else(|err| panic!("invalid built-in CharactersTofu.txt: {err}"))
            .into_iter()
            .map(|(tofu, fallback, level)| (tofu, (fallback, level)))
            .collect()
    })
}

fn detofu_builtin_into(input: &str, level: DetofuLevel, output: &mut String) {
    let map = tofu_map();
    output.reserve(input.len());

    for ch in input.chars() {
        // The built-in DeTofu table starts at CJK Extension B (U+20000).
        // Ordinary BMP text therefore bypasses hashing completely.
        if ch < '\u{20000}' {
            output.push(ch);
            continue;
        }

        match map.get(&ch) {
            Some(&(fallback, entry_level)) if entry_level >= level => output.push(fallback),
            _ => output.push(ch),
        }
    }
}

/// A reusable, customizable DeTofu display-compatibility map.
///
/// `DetofuMap` combines:
///
/// - one process-wide immutable built-in fallback table; and
/// - a small owned overlay containing only application-specific entries.
///
/// Custom entries take precedence over built-in entries. Creating a
/// `DetofuMap` does not clone or filter the built-in table.
///
/// DeTofu is independent of OpenCC conversion dictionaries. It does not
/// participate in Simplified/Traditional phrase matching, regional variant
/// selection, punctuation conversion, or other OpenCC conversion logic.
///
/// For normal conversion workflows, prefer [`OpenCC::detofu`](crate::OpenCC::detofu)
/// as the post-conversion DeTofu step. Use `DetofuMap` directly when you need a
/// reusable map or application-specific fallback overrides.
///
/// # Examples
///
/// ```rust
/// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
///
/// let map = DetofuMap::builtin(DetofuLevel::ExtB)
///     .with_custom_pairs(&[('𣭲', '氄')]);
///
/// let safe = map.detofu("這隻小狗有𣭲毛");
///
/// assert_eq!(safe, "這隻小狗有氄毛");
/// ```
#[derive(Debug, Clone)]
pub struct DetofuMap {
    level: DetofuLevel,
    custom: FxHashMap<char, char>,
}

impl DetofuMap {
    /// Creates a reusable DeTofu map backed by the shared built-in table.
    ///
    /// The selected [`DetofuLevel`] is threshold-based. For example,
    /// [`DetofuLevel::ExtB`] enables all supported non-BMP mappings, while
    /// [`DetofuLevel::ExtE`] enables only ExtE and later supported mappings.
    ///
    /// This constructor does not clone, filter, or allocate a private copy of
    /// the built-in table. The returned value initially contains an empty
    /// custom overlay.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB);
    ///
    /// assert_eq!(map.detofu("骖𬴂"), "骖騑");
    /// ```
    pub fn builtin(level: DetofuLevel) -> Self {
        Self {
            level,
            custom: FxHashMap::default(),
        }
    }

    /// Adds or overrides compatibility fallback entries from a mapping file.
    ///
    /// The file uses the same tab-separated format as the built-in generated
    /// data:
    ///
    /// ```text
    /// source_character<TAB>fallback_character<TAB>extension
    /// ```
    ///
    /// The extension field accepts compact forms such as `B`, full forms such
    /// as `ExtB`, and hyphenated forms such as `ext-b`. Parsing is ASCII
    /// case-insensitive.
    ///
    /// Blank lines and lines beginning with `#` are ignored. Malformed entries,
    /// missing fields, or unsupported extension values return
    /// [`std::io::ErrorKind::InvalidData`] with the source line number.
    ///
    /// File entries below this map's threshold are ignored. Eligible entries
    /// are stored only in this map's custom overlay and take precedence over
    /// the shared built-in table.
    pub fn with_custom_file<P: AsRef<Path>>(mut self, path: P) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;

        for (tofu, fallback, entry_level) in parse_tofu_entries(&text)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?
        {
            if entry_level >= self.level {
                self.custom.insert(tofu, fallback);
            }
        }

        Ok(self)
    }

    /// Adds or overrides application-specific fallback pairs.
    ///
    /// Custom pairs take precedence over the shared built-in table. Unlike
    /// file entries, direct pairs have no extension metadata and are therefore
    /// always active, regardless of this map's [`DetofuLevel`].
    ///
    /// Only the supplied pairs are stored in this `DetofuMap`; the built-in
    /// table remains immutable and shared.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB)
    ///     .with_custom_pairs(&[('𣭲', '氄')]);
    ///
    /// assert_eq!(map.detofu("𣭲"), "氄");
    /// ```
    ///
    /// A direct pair can override a built-in mapping:
    ///
    /// ```rust
    /// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB)
    ///     .with_custom_pairs(&[('𬴂', '馬')]);
    ///
    /// assert_eq!(map.detofu("𬴂"), "馬");
    /// ```
    pub fn with_custom_pairs(mut self, pairs: &[(char, char)]) -> Self {
        self.custom.extend(pairs.iter().copied());
        self
    }

    /// Applies this map and appends the result to an existing [`String`].
    ///
    /// Custom entries are checked first. If no custom entry exists, the shared
    /// built-in table is consulted and this map's [`DetofuLevel`] threshold is
    /// applied. Characters without an eligible mapping are copied unchanged.
    ///
    /// This method appends to `output`; it does not clear existing contents.
    /// Call [`String::clear`] first when reusing a buffer for an independent
    /// result.
    ///
    /// When no custom entries are present, this method automatically uses the
    /// optimized built-in-only path, including the BMP fast path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB)
    ///     .with_custom_pairs(&[('𣭲', '氄')]);
    ///
    /// let mut output = String::from("結果：");
    /// map.detofu_into("𣭲毛", &mut output);
    ///
    /// assert_eq!(output, "結果：氄毛");
    /// ```
    pub fn detofu_into(&self, input: &str, output: &mut String) {
        if self.custom.is_empty() {
            detofu_builtin_into(input, self.level, output);
            return;
        }

        output.reserve(input.len());
        let builtins = tofu_map();

        for ch in input.chars() {
            if let Some(&fallback) = self.custom.get(&ch) {
                output.push(fallback);
                continue;
            }

            // Direct custom pairs may target BMP characters, so they must be
            // checked first. After a custom miss, BMP characters can safely
            // bypass the built-in table.
            if ch < '\u{20000}' {
                output.push(ch);
                continue;
            }

            match builtins.get(&ch) {
                Some(&(fallback, entry_level)) if entry_level >= self.level => {
                    output.push(fallback);
                }
                _ => output.push(ch),
            }
        }
    }

    /// Applies this map and returns a newly allocated result [`String`].
    ///
    /// Custom entries take precedence over the shared built-in table.
    /// Characters without an eligible mapping are copied unchanged.
    ///
    /// Use [`DetofuMap::detofu_into`] when processing multiple inputs and
    /// reusing an output buffer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::{DetofuLevel, DetofuMap};
    ///
    /// let map = DetofuMap::builtin(DetofuLevel::ExtB)
    ///     .with_custom_pairs(&[('𣭲', '氄')]);
    ///
    /// assert_eq!(map.detofu("這隻小狗有𣭲毛"), "這隻小狗有氄毛");
    /// ```
    pub fn detofu(&self, input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        self.detofu_into(input, &mut output);
        output
    }
}

/// Applies the built-in DeTofu table and appends the result to `output`.
///
/// This is the internal built-in-only path used by higher-level APIs.
/// Existing output is preserved.
pub(crate) fn detofu_into(input: &str, level: DetofuLevel, output: &mut String) {
    detofu_builtin_into(input, level, output);
}

/// Applies the built-in DeTofu table and returns a newly allocated result.
///
/// This is the internal built-in-only convenience path used by higher-level
/// APIs.
pub(crate) fn detofu(input: &str, level: DetofuLevel) -> String {
    let mut output = String::with_capacity(input.len());
    detofu_builtin_into(input, level, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_level_aliases() {
        assert_eq!(DetofuLevel::parse("all"), Ok(DetofuLevel::ExtB));
        assert_eq!(DetofuLevel::parse(" Ext-C "), Ok(DetofuLevel::ExtC));
        assert_eq!(DetofuLevel::parse("i"), Ok(DetofuLevel::ExtI));
    }

    #[test]
    fn builtin_detofu_replaces_known_mapping() {
        assert_eq!(detofu("骖𬴂", DetofuLevel::ExtB), "骖騑");
    }

    #[test]
    fn builtin_detofu_preserves_bmp_text() {
        assert_eq!(
            detofu("普通中文 ABC 123", DetofuLevel::ExtB),
            "普通中文 ABC 123"
        );
    }

    #[test]
    fn free_detofu_into_appends() {
        let mut output = String::from("結果：");
        detofu_into("𬴂", DetofuLevel::ExtB, &mut output);
        assert_eq!(output, "結果：騑");
    }

    #[test]
    fn direct_custom_pair_overrides_builtin_mapping() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𬴂', '馬')]);

        assert_eq!(map.detofu("𬴂"), "馬");
    }

    #[test]
    fn direct_custom_pair_can_target_bmp_character() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('A', 'B')]);

        assert_eq!(map.detofu("A𬴂"), "B騑");
    }

    #[test]
    fn map_detofu_into_reuses_output_buffer() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB).with_custom_pairs(&[('𣭲', '氄')]);
        let mut output = String::with_capacity(128);

        map.detofu_into("𣭲毛", &mut output);
        assert_eq!(output, "氄毛");

        output.clear();
        map.detofu_into("𣭲𣭲", &mut output);
        assert_eq!(output, "氄氄");
    }

    #[test]
    fn plain_builtin_map_uses_shared_table() {
        let map = DetofuMap::builtin(DetofuLevel::ExtB);
        assert!(map.custom.is_empty());
        assert_eq!(map.detofu("骖𬴂"), "骖騑");
    }
}
