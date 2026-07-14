//! # opencc-jieba-rs
//!
//! `opencc-jieba-rs` is a high-performance Rust library for Chinese text conversion,
//! segmentation, and keyword extraction. It integrates [Jieba](https://github.com/fxsjy/jieba) for word segmentation
//! and a multi-stage OpenCC-style dictionary system for converting between different Chinese variants.
//!
//! ## Features
//!
//! - Simplified ↔ Traditional Chinese conversion (including Taiwan, Hong Kong, Japanese variants)
//! - Multi-pass dictionary-based phrase replacement
//! - Fast and accurate word segmentation using Jieba
//! - Jieba user dictionary loading with [`OpenCC::load_user_dict`],
//!   [`OpenCC::try_new_with_user_dict_path`], and [`OpenCC::new_with_user_dict`]
//! - Keyword extraction using TF-IDF or TextRank
//! - Optional punctuation conversion (e.g., 「」 ↔ “”)
//!
//! ## Example
//!
//! ```rust
//! use opencc_jieba_rs::OpenCC;
//!
//! let opencc = OpenCC::new();
//! let s = opencc.s2t("“春眠不觉晓，处处闻啼鸟。”", true);
//! println!("{}", s); // -> "「春眠不覺曉，處處聞啼鳥。」"
//! ```
//!
//! ## Use Cases
//!
//! - Text normalization for NLP and search engines
//! - Cross-regional Chinese content adaptation
//! - Automatic subtitle or document localization
//!
//! ## Crate Status
//!
//! - 🚀 Fast and parallelized
//! - 🧪 Battle-tested on multi-million character corpora
//! - 📦 Ready for crates.io and docs.rs publication
//!
//! ---
//! # Conversion Overview (OpenCC + Jieba)
//!
//! `opencc_jieba_rs::OpenCC` provides a set of high-level helpers that mirror
//! common OpenCC configurations, built on top of:
//!
//! - **OpenCC dictionaries** (character / phrase mappings)
//! - **Jieba segmentation** for phrase-level matching
//! - Optional **punctuation conversion**
//!
//! All methods take `&self` and `&str` input and return a newly allocated
//! `String`.
//!
//! ## Quick Start
//!
//! ```rust
//! let opencc = opencc_jieba_rs::OpenCC::new();
//!
//! let s = "这里进行着“汉字转换”测试。";
//! let t = opencc.s2t(s, false);       // Simplified → Traditional (phrase-level)
//! let tw = opencc.t2tw(&t);    // Traditional → Taiwan Traditional
//! ```
//!
//! ## Phrase-Level vs Character-Level
//!
//! There are two main categories of conversion:
//!
//! 1. **Phrase-level conversions**
//!    Use Jieba segmentation and multiple dictionaries to correctly handle
//!    idioms, multi-character words, and regional preferences.
//!
//! 2. **Character-level conversions**
//!    Use only character variant dictionaries (no segmentation), ideal for
//!    high-speed normalization where phrase context is unimportant.
//!
//! ## Core Simplified ↔ Traditional
//!
//! | Direction | Method         | Level      | Notes                                     |
//! |----------|----------------|-----------|-------------------------------------------|
//! | S → T    | [`OpenCC::s2t`] | Phrase    | Standard Simplified → Traditional.        |
//! | T → S    | [`OpenCC::t2s`] | Phrase    | Standard Traditional → Simplified.        |
//! | S → T    | `st`            | Character | Fast char-only S→T (no segmentation).     |
//! | T → S    | `ts`            | Character | Fast char-only T→S (no segmentation).     |
//!
//! ### `s2t` / `t2s`
//!
//! - Use phrase dictionaries + Jieba segmentation.
//! - Preserve idioms and phrase-level semantics where possible.
//! - Recommended for user-facing text conversion.
//!
//! ### `st` / `ts`
//!
//! - Use only `st_characters` / `ts_characters` dictionaries.
//! - Do **not** segment or match phrases.
//! - Ideal for:
//!   - bulk normalization
//!   - preprocessing before heavier conversions
//!
//! ## Taiwan Traditional (Tw)
//!
//! | Direction      | Method             | Description                                               |
//! |----------------|--------------------|-----------------------------------------------------------|
//! | T → Tw         | [`OpenCC::t2tw`]   | Standard Traditional → Taiwan variants.                  |
//! | T → Tw (phr.)  | [`OpenCC::t2twp`]  | T→Tw with Taiwan phrase and variant preferences.         |
//! | Tw → T         | [`OpenCC::tw2t`]   | Taiwan variants → Standard Traditional.                  |
//! | Tw → T (phr.)  | [`OpenCC::tw2tp`]  | Tw→T with additional reverse phrase normalization.       |
//!
//! - `t2tw` uses `tw_variants_phrases` + `tw_variants` for Taiwan-specific forms.
//! - `t2twp` uses one ordered pass: `tw_phrases`, `tw_variants_phrases`, then
//!   `tw_variants`. The first matching dictionary wins.
//! - `tw2t` and `tw2tp` are reverse directions. `tw2tp` likewise uses one
//!   ordered pass: `tw_variants_rev`, `tw_variants_rev_phrases`, then
//!   `tw_phrases_rev`.
//!
//! ## Hong Kong Traditional (HK)
//!
//! | Direction      | Method              | Description                                          |
//! |----------------|---------------------|------------------------------------------------------|
//! | T → HK         | [`OpenCC::t2hk`]    | Standard Traditional → Hong Kong variants.          |
//! | T → HK (phr.)  | [`OpenCC::t2hkp`]   | T→HK with Hong Kong phrase and variant preferences.  |
//! | HK → T         | [`OpenCC::hk2t`]    | Hong Kong variants → Standard Traditional.          |
//! | HK → T (phr.)  | [`OpenCC::hk2tp`]   | HK→T with reverse phrase normalization.              |
//! | S → HKP        | [`OpenCC::s2hkp`]   | Simplified → Hong Kong with phrase preferences.     |
//! | HKP → S        | [`OpenCC::hk2sp`]   | Hong Kong phrases → Simplified.                     |
//!
//! - `t2hk` applies `hk_variants_phrases` + `hk_variants` (HK-specific variants and preferences).
//! - `hk2t` uses `hk_variants_rev_phrases` + `hk_variants_rev` to normalize
//!   back to standard Traditional.
//! - `t2hkp` and `hk2tp` add `hk_phrases` or `hk_phrases_rev` in the same
//!   single ordered pass; the first matching dictionary wins.
//! - `s2hkp` and `hk2sp` additionally apply `hk_phrases` or
//!   `hk_phrases_rev` in their regional phrase round.
//!
//! ## Japanese Kanji (Shinjitai / Kyūjitai)
//!
//! | Direction | Method             | Description                                                  |
//! |-----------|--------------------|--------------------------------------------------------------|
//! | T → JP    | [`OpenCC::t2jp`]   | Traditional → Japanese Shinjitai-like variants (Kanji).     |
//! | JP → T    | [`OpenCC::jp2t`]   | Japanese Shinjitai → Traditional (Kyūjitai-style) mapping.  |
//!
//! - `t2jp` uses `jps_characters_rev` to map Traditional forms to standard
//!   Japanese Shinjitai (e.g. 體 → 体, 圖 → 図 where applicable).
//! - `jp2t` combines `jps_phrases` and `jps_characters` to reverse these
//!   mappings back to Traditional Chinese.
//!
//! ## Punctuation and Symbols
//!
//! Most high-level methods enable **punctuation conversion** by default,
//! using OpenCC’s punctuation dictionaries to normalize:
//!
//! - Chinese-style quotes / brackets
//! - Full-width / half-width punctuation
//!
//! Lower-level helpers inside this crate may expose more granular control if
//! you need to:
//!
//! - disable punctuation conversion
//! - run custom dictionary pipelines
//! - integrate with your own segmentation logic
//!
//! ## User Dictionaries
//!
//! Jieba user dictionaries can be loaded during construction or added later to
//! an existing [`OpenCC`] instance. Entries use the format:
//!
//! ```text
//! word freq [tag]
//! ```
//!
//! The `freq` field is required and must be a valid integer. The POS `tag`
//! field is optional. Lines containing only `word`, or `word tag` without an
//! integer frequency, are rejected before data is passed to `jieba-rs`.
//!
//! ```no_run
//! use opencc_jieba_rs::OpenCC;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cc = OpenCC::try_new_with_user_dict_path("dicts/user_dict.txt")?;
//! let words = cc.jieba_cut("OpenAI和云计算", false);
//! # Ok(())
//! # }
//! ```
//!
//! To load several dictionaries in order:
//!
//! ```no_run
//! use opencc_jieba_rs::OpenCC;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut cc = OpenCC::new();
//! cc.load_user_dict("dicts/user_dict.txt")?;
//! cc.load_user_dict("dicts/domain_terms.txt")?;
//! # Ok(())
//! # }
//! ```
//!
//! `new_with_user_dict()` is a convenience wrapper that loads
//! `dicts/user_dict.txt`.
//!
//! ## Custom Conversion Dictionaries
//!
//! Zstd-compressed conversion packs generated by the workspace
//! `dict-generate` tool can replace the built-in OpenCC mappings at runtime.
//! This API is available without the `dictionary-build` feature:
//!
//! ```no_run
//! use opencc_jieba_rs::OpenCC;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut cc = OpenCC::try_new_with_dictionary_zstd("dictionary.json.zst")?;
//! cc.load_user_dict("dicts/user_dict.txt")?;
//! # Ok(())
//! # }
//! ```
//!
//! See [`OpenCC::load_dictionary_zstd`] to replace the conversion pack on an
//! existing instance.
//!
//! ## When to Use What?
//!
//! - Use **`s2t` / `t2s`** for general purpose Simplified/Traditional
//!   conversion.
//! - Use **`t2tw` / `t2twp` / `tw2t` / `tw2tp`** when targeting **Taiwan**
//!   content or normalizing it.
//! - Use **`t2hk` / `t2hkp` / `hk2t` / `hk2tp`** for Hong Kong variants, and
//!   **`s2hkp` / `hk2sp`** when Hong Kong phrase preferences are required.
//! - Use **`t2jp` / `jp2t`** for interoperability with **Japanese Kanji** forms,
//!   when only character-shape conversion is desired (not full translation).
//! - Use **`st` / `ts`** when you need **fast, character-only** normalization
//!   with minimal overhead.
//!
//! For segmentation-only or keyword extraction APIs, see:
//!
//! - [`OpenCC::jieba_cut`] — Jieba segmentation (accurate mode)
//! - [`OpenCC::jieba_cut_for_search`] — Jieba segmentation optimized for search indexing
//! - [`OpenCC::jieba_cut_all`] — Jieba full segmentation mode
//! - [`OpenCC::keyword_extract_textrank`] — keyword extraction using TextRank
//! - [`OpenCC::keyword_extract_tfidf`] — keyword extraction using TF-IDF
//!
//! These utilities can be used independently of Chinese variant conversion,
//! or combined with [`OpenCC::convert`] results for downstream NLP tasks such
//! as indexing, text analysis, and keyword extraction.
use jieba_rs::Jieba;
use rayon::prelude::*;
use regex::Regex;
use std::borrow::Cow;
use std::fs::File;
use std::io::Cursor;
use std::io::{BufRead, BufReader, Read};
use std::ops::Range;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::{fmt, io};
use zstd::stream::read::Decoder;

use crate::dictionary_lib::{DictMap, Dictionary};
use crate::keyword::keyword_extract_internal;
mod dictionary_lib;

/// Dictionary-pack generation operations.
///
/// This module is available only with the `dictionary-build` feature. It is
/// intended for tooling that builds runtime packs from the repository's
/// `./dicts` source directory. The runtime dictionary representation, maps,
/// metadata, and logical slots remain private.
///
/// The plaintext dictionary sources are not included in the published crate,
/// so callers must provide the expected `./dicts` directory themselves.
#[cfg(feature = "dictionary-build")]
pub mod dictionary_build {
    use crate::dictionary_lib::Dictionary;
    use std::io;
    use std::path::Path;

    /// Builds dictionaries from `./dicts` and writes pretty-printed JSON.
    pub fn write_json_pretty(path: impl AsRef<Path>) -> io::Result<()> {
        Dictionary::from_dicts().save_json(path, true)
    }

    /// Builds dictionaries from `./dicts` and writes Zstd-compressed JSON.
    pub fn write_zstd(path: impl AsRef<Path>) -> io::Result<()> {
        Dictionary::from_dicts().save_json_compressed(path)
    }
}
mod keyword;
pub use keyword::{KeywordMethod, POS_KEYWORDS};
mod opencc_config;
pub use jieba_rs::Keyword;
pub use opencc_config::OpenccConfig;

const DICT_HANS_HANT_ZSTD: &[u8] = include_bytes!("dictionary_lib/dicts/dict_hans_hant.txt.zst");

/// Master delimiter string containing all punctuation and whitespace treated
/// as token boundaries by OpenCC-Jieba.
///
/// This includes:
///
/// - ASCII whitespace such as space, tab, carriage return, and line feed
/// - ASCII punctuation and symbols
/// - Common CJK punctuation such as `、。『』「」《》【】`
/// - Fullwidth punctuation such as `，；：？！～`
///
/// The string is used to build a compact bitmap for fast delimiter checks.
/// Most callers should use [`is_delimiter`] instead of referencing this
/// constant directly.
const DELIMITER_STR: &str =
    " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";

/// Lazily initialized delimiter bitmap for all BMP code points
/// (`U+0000..=U+FFFF`).
///
/// The bitmap contains 65,536 bits stored as 1024 `u64` words. A set bit
/// means the corresponding character is treated as a delimiter by
/// [`is_delimiter`].
static DELIM_BMP: OnceLock<[u64; 1024]> = OnceLock::new();

/// Returns the global delimiter bitmap, initializing it on first use.
///
/// Initialization is performed once for the lifetime of the program.
/// Subsequent calls return a shared reference to the precomputed bitmap.
#[inline(always)]
fn delim_bmp() -> &'static [u64; 1024] {
    DELIM_BMP.get_or_init(|| {
        let mut bm = [0u64; 1024];
        for ch in DELIMITER_STR.chars() {
            let u = ch as u32;
            if u <= 0x_FFFF {
                let idx = (u >> 6) as usize;
                let bit = u & 63;
                bm[idx] |= 1u64 << bit;
            }
        }
        bm
    })
}

/// Returns `true` if `c` is treated as a delimiter.
///
/// This function performs a constant-time bitmap lookup for any BMP character
/// (`U+0000..=U+FFFF`). Non-BMP code points currently return `false`, since
/// the delimiter table only covers the BMP.
///
/// # Arguments
///
/// * `c` - The character to test.
///
/// # Returns
///
/// - `true` if `c` is considered a delimiter
/// - `false` otherwise
///
/// # Performance
///
/// This is a very fast hot-path check:
///
/// - one bitmap access
/// - one bit test
/// - no hashing or heap allocation
///
/// The bitmap is initialized once and then reused for the rest of the
/// process lifetime.
///
/// # Examples
///
/// ```
/// use opencc_jieba_rs::is_delimiter;
///
/// assert!(is_delimiter('。'));
/// assert!(is_delimiter(' '));
/// assert!(!is_delimiter('你'));
/// ```
#[inline(always)]
pub fn is_delimiter(c: char) -> bool {
    let u = c as u32;
    if u <= 0x_FFFF {
        let bm = delim_bmp();
        let word = unsafe { *bm.get_unchecked((u >> 6) as usize) };
        ((word >> (u & 63)) & 1) != 0
    } else {
        false
    }
}

/// Lazily initialized regex used by [`OpenCC::zho_check`] to strip
/// non-Chinese content before heuristic classification.
///
/// This regex is compiled once on first use and then reused for the lifetime
/// of the program.
static STRIP_REGEX: OnceLock<Regex> = OnceLock::new();

#[inline]
fn strip_regex() -> &'static Regex {
    STRIP_REGEX.get_or_init(|| {
        Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]")
            .expect("STRIP_REGEX must be a valid regex")
    })
}

// Minimum input length (in chars) to trigger parallel processing
const PARALLEL_THRESHOLD: usize = 1000;

/// A slice of dictionary references used in a single conversion round.
///
/// Each round consists of one or more [`DictMap`] dictionaries that are
/// applied in priority order during phrase conversion.
///
/// The first dictionary that matches a phrase wins.
///
/// This is an internal helper type used by [`OpenCC::convert_rounds`]
/// to represent a single stage of dictionary-based conversion.
///
/// # Example
///
/// ```ignore
/// let round = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
/// ```
type DictRefs<'a> = &'a [&'a DictMap];

/// A sequence of dictionary rounds used in a multi-stage conversion pipeline.
///
/// Each element in the slice represents one conversion stage. The output of
/// each round becomes the input of the next.
///
/// Internally this allows OpenCC configurations such as:
///
/// ```text
/// input
///   ↓ round1 (phrase dictionaries)
///   ↓ round2 (variant dictionaries)
///   ↓ round3 (final normalization)
/// output
/// ```
///
/// This type is used internally by [`OpenCC::convert_rounds`] to simplify
/// multi-pass dictionary pipelines like `s2twp`, `tw2sp`, and `hk2s`.
type DictRounds<'a> = &'a [DictRefs<'a>];

/// Error type returned by fallible [`OpenCC`] constructors and dictionary-loading APIs.
///
/// This error currently covers:
///
/// - embedded Jieba dictionary initialization
/// - embedded zstd dictionary decoding
/// - custom OpenCC conversion dictionary loading
/// - user dictionary file I/O
/// - user dictionary parsing
///
/// # Examples
///
/// ```no_run
/// use opencc_jieba_rs::OpenCC;
///
/// let opencc = OpenCC::try_new_with_user_dict_path("dicts/user_dict.txt")?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub enum OpenccError {
    /// Failed to create or read the zstd decoder for the embedded Jieba dictionary.
    ZstdDecode(String),

    /// Failed to initialize the embedded Jieba tokenizer.
    JiebaInit(String),

    /// Failed to open or read a custom OpenCC conversion dictionary pack.
    DictionaryIo(io::Error),

    /// Failed to decompress a custom OpenCC conversion dictionary pack.
    DictionaryDecode(String),

    /// Failed to deserialize a custom OpenCC conversion dictionary pack.
    DictionaryParse(String),

    /// The custom OpenCC conversion dictionary uses a newer unsupported schema.
    UnsupportedDictionarySchema {
        /// Schema version found in the custom dictionary pack.
        found: u16,
        /// Newest schema version supported by this crate release.
        max_supported: u16,
    },

    /// Failed to open or read a Jieba user dictionary file.
    UserDictIo(io::Error),

    /// Failed to parse a Jieba user dictionary entry.
    ///
    /// User dictionaries should use the jieba-rs format:
    ///
    /// ```text
    /// word freq [tag]
    /// ```
    ///
    /// Example:
    ///
    /// ```text
    /// 云计算 100000 n
    /// 蓝翔 100000 nz
    /// 区块链 10 nz
    /// ```
    UserDictParse(String),
}

impl fmt::Display for OpenccError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZstdDecode(e) => write!(f, "zstd decode error: {e}"),
            Self::JiebaInit(e) => write!(f, "jieba initialization error: {e}"),
            Self::DictionaryIo(e) => write!(f, "conversion dictionary I/O error: {e}"),
            Self::DictionaryDecode(e) => {
                write!(f, "conversion dictionary decode error: {e}")
            }
            Self::DictionaryParse(e) => {
                write!(f, "conversion dictionary parse error: {e}")
            }
            Self::UnsupportedDictionarySchema {
                found,
                max_supported,
            } => write!(
                f,
                "unsupported conversion dictionary schema {found}; maximum supported is {max_supported}"
            ),
            Self::UserDictIo(e) => write!(f, "user dictionary I/O error: {e}"),
            Self::UserDictParse(e) => write!(f, "user dictionary parse error: {e}"),
        }
    }
}

impl std::error::Error for OpenccError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DictionaryIo(e) | Self::UserDictIo(e) => Some(e),
            _ => None,
        }
    }
}

/// The main struct for performing Chinese text conversion and segmentation.
///
/// `OpenCC` combines a [`Jieba`] tokenizer with OpenCC-style dictionaries,
/// allowing high-quality conversion between Simplified, Traditional, Taiwanese,
/// Hong Kong, and Japanese variants of Chinese. It also supports keyword extraction
/// and multi-stage phrase replacement.
///
/// # Example
///
/// ```rust
/// use opencc_jieba_rs::OpenCC;
///
/// let opencc = OpenCC::new();
/// let result = opencc.s2t("“春眠不觉晓，处处闻啼鸟。”", true);
/// assert_eq!(result, "「春眠不覺曉，處處聞啼鳥。」");
/// ```
///
/// # Features
///
/// - Supports segmentation with Jieba (HMM on/off)
/// - Dictionary-based multi-pass phrase replacement
/// - Conversion between: Simplified ↔ Traditional, Taiwan, HK, Japanese
/// - Optional punctuation conversion (e.g., 「」 vs “”) and keyword extraction
///
/// [`Jieba`]: https://docs.rs/jieba-rs
pub struct OpenCC {
    /// The Jieba tokenizer instance.
    pub jieba: Arc<Jieba>,
    /// The conversion dictionary.
    dictionary: Dictionary,
}

impl OpenCC {
    /// Creates a new instance of `OpenCC` with built-in dictionaries and a Jieba tokenizer.
    ///
    /// Loads the default compressed dictionary for Simplified-Traditional conversion,
    /// initializes the Jieba tokenizer, and prepares the dictionary engine.
    ///
    /// # Panics
    ///
    /// Panics if the internal Jieba dictionary fails to load.
    ///
    /// # Example
    /// ```
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// let opencc = OpenCC::new();
    /// ```
    pub fn new() -> Self {
        Self::try_new_internal().expect("failed to init OpenCC")
    }

    /// Creates an [`OpenCC`] instance with a custom Zstd-compressed conversion
    /// dictionary pack.
    ///
    /// Packs produced by the workspace `dict-generate` tool are accepted. This
    /// changes the OpenCC character and phrase mappings only; the bundled Jieba
    /// tokenizer dictionary remains in use.
    ///
    /// A Jieba user dictionary can be added afterward with
    /// [`OpenCC::load_user_dict`]:
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut cc = OpenCC::try_new_with_dictionary_zstd("dictionary.json.zst")?;
    /// cc.load_user_dict("dicts/user_dict.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, the Zstd or JSON data is
    /// invalid, the pack uses a newer unsupported schema, or Jieba cannot be
    /// initialized.
    pub fn try_new_with_dictionary_zstd<P: AsRef<Path>>(path: P) -> Result<Self, OpenccError> {
        let dictionary = Self::read_dictionary_zstd(path)?;
        Self::try_new_with_dictionary(dictionary)
    }

    /// Replaces this instance's OpenCC conversion mappings from a custom
    /// Zstd-compressed dictionary pack.
    ///
    /// This method composes with [`OpenCC::try_new_with_user_dict_path`] and
    /// [`OpenCC::new_with_user_dict`]. The existing dictionary remains unchanged
    /// if loading or validation fails.
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut cc = OpenCC::try_new_with_user_dict_path("dicts/user_dict.txt")?;
    /// cc.load_dictionary_zstd("dictionary.json.zst")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, the Zstd or JSON data is
    /// invalid, or the pack uses a newer unsupported schema.
    pub fn load_dictionary_zstd<P: AsRef<Path>>(&mut self, path: P) -> Result<(), OpenccError> {
        let dictionary = Self::read_dictionary_zstd(path)?;
        self.dictionary = dictionary;
        Ok(())
    }

    /// Loads a Jieba user dictionary into the current [`OpenCC`] instance.
    ///
    /// This method mutates the internal Jieba tokenizer by merging entries from
    /// the provided user dictionary file. Newly loaded entries may override or
    /// influence segmentation behavior.
    ///
    /// # Lifecycle
    ///
    /// This method requires exclusive access to the internal tokenizer and must be
    /// called **before** the [`OpenCC`] instance is shared (e.g. wrapped in [`Arc`]
    /// or used across threads).
    ///
    /// # User dictionary format
    ///
    /// The file must follow the `jieba-rs` format:
    ///
    /// ```text
    /// word freq [tag]
    /// ```
    ///
    /// Example:
    ///
    /// ```text
    /// 云计算 100000 n
    /// 蓝翔 100000 nz
    /// 区块链 10 nz
    /// ```
    ///
    /// > Note: `freq` is always required and must be a valid integer.
    /// > `tag` is optional. Formats like `word` or `word tag` are **not supported**.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - the file cannot be opened
    /// - the dictionary format is invalid
    /// - the internal tokenizer is already shared
    ///
    /// # Notes
    ///
    /// This method must be called before the instance is shared across threads.
    /// After sharing, the tokenizer becomes immutable.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut cc = OpenCC::new();
    /// cc.load_user_dict("dicts/user_dict.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_user_dict<P: AsRef<Path>>(&mut self, path: P) -> Result<(), OpenccError> {
        let file = File::open(path).map_err(OpenccError::UserDictIo)?;
        let reader = BufReader::new(file);

        let validated = validate_user_dict_format(reader)?;
        let mut reader = BufReader::new(validated.as_bytes());

        // 1. clone current jieba (cheap enough, done rarely)
        let mut new_jieba = (*self.jieba).clone();

        // 2. apply changes to clone
        new_jieba
            .load_dict(&mut reader)
            .map_err(|e| OpenccError::UserDictParse(e.to_string()))?;

        // 3. swap only if success
        self.jieba = Arc::new(new_jieba);

        Ok(())
    }

    /// Internal fallible constructor for [`OpenCC`].
    ///
    /// This method initializes the embedded Jieba tokenizer from the compressed
    /// dictionary bundled with the crate. It performs all setup without panicking
    /// and returns a [`Result`] instead.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - the embedded zstd dictionary cannot be decoded
    /// - the embedded Jieba dictionary is invalid or fails to initialize
    ///
    /// # Notes
    ///
    /// This is an internal helper used by fallible constructors such as
    /// [`OpenCC::try_new_with_user_dict_path`]. Most users should call
    /// [`OpenCC::new`] or other public constructors instead.
    ///
    /// The returned instance contains only the built-in dictionary. User
    /// dictionaries can be loaded later via [`OpenCC::load_user_dict`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Normally not called directly; shown here for completeness
    /// let cc = OpenCC::try_new_with_user_dict_path("dicts/user_dict.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    fn try_new_internal() -> Result<Self, OpenccError> {
        Self::try_new_with_dictionary(Dictionary::new())
    }

    fn try_new_with_dictionary(dictionary: Dictionary) -> Result<Self, OpenccError> {
        let cursor = Cursor::new(DICT_HANS_HANT_ZSTD);

        let decoder = Decoder::new(cursor).map_err(|e| OpenccError::ZstdDecode(e.to_string()))?;

        let mut buf = BufReader::new(decoder);

        let jieba =
            Jieba::with_dict(&mut buf).map_err(|e| OpenccError::JiebaInit(e.to_string()))?;

        Ok(OpenCC {
            jieba: Arc::new(jieba),
            dictionary,
        })
    }

    fn read_dictionary_zstd<P: AsRef<Path>>(path: P) -> Result<Dictionary, OpenccError> {
        let file = File::open(path).map_err(OpenccError::DictionaryIo)?;
        let mut decoder =
            Decoder::new(file).map_err(|error| OpenccError::DictionaryDecode(error.to_string()))?;
        let mut json = String::new();
        decoder
            .read_to_string(&mut json)
            .map_err(|error| OpenccError::DictionaryDecode(error.to_string()))?;

        let dictionary: Dictionary = serde_json::from_str(&json)
            .map_err(|error| OpenccError::DictionaryParse(error.to_string()))?;

        if dictionary.schema_version > dictionary_lib::SCHEMA_VERSION {
            return Err(OpenccError::UnsupportedDictionarySchema {
                found: dictionary.schema_version,
                max_supported: dictionary_lib::SCHEMA_VERSION,
            });
        }

        Ok(dictionary)
    }

    /// Creates a new [`OpenCC`] instance and loads a user dictionary from the given path.
    ///
    /// This is a convenience constructor equivalent to:
    ///
    /// ```no_run
    /// # use opencc_jieba_rs::OpenCC;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut cc = OpenCC::new();
    /// cc.load_user_dict("dicts/user_dict.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - the embedded Jieba dictionary fails to initialize
    /// - the user dictionary file cannot be opened
    /// - the user dictionary format is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cc = OpenCC::try_new_with_user_dict_path("dicts/user_dict.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn try_new_with_user_dict_path<P: AsRef<Path>>(path: P) -> Result<Self, OpenccError> {
        let mut opencc = Self::try_new_internal()?;
        opencc.load_user_dict(path)?;
        Ok(opencc)
    }

    /// Creates a new [`OpenCC`] instance and loads a user dictionary from the
    /// default path `dicts/user_dict.txt`.
    ///
    /// This is a convenience wrapper around
    /// [`OpenCC::try_new_with_user_dict_path`].
    ///
    /// # Behavior
    ///
    /// - If the file exists, it will be loaded
    /// - If the file is missing or invalid, an error is returned
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - the embedded Jieba dictionary fails to initialize
    /// - the default user dictionary cannot be opened
    /// - the user dictionary format is invalid
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cc = OpenCC::new_with_user_dict()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_with_user_dict() -> Result<Self, OpenccError> {
        Self::try_new_with_user_dict_path("dicts/user_dict.txt")
    }

    /// Performs dictionary-based phrase-level conversion with character-level fallback.
    ///
    /// This is the core logic for converting text using Jieba segmentation and one or more
    /// dictionaries. Tokens are matched against phrase dictionaries in priority order, with
    /// a per-character fallback when no phrase match exists.
    ///
    /// ## Workflow
    /// 1. Split the input into ranges based on delimiters (punctuation, whitespace, etc.).
    /// 2. For each range, segment with Jieba (`cut`) with or without HMM.
    /// 3. For each token:
    ///    - If it is a known delimiter, return it unchanged.
    ///    - Otherwise, look it up in each dictionary (short-circuiting on the first match).
    ///    - If no match is found, fall back to character-by-character conversion (in-place).
    ///
    /// ## Parallelism
    /// - If the input length ≥ `PARALLEL_THRESHOLD`, ranges are processed in parallel
    ///   and concatenated into the output.
    /// - For shorter inputs, ranges are processed serially to avoid overhead.
    ///
    /// # Arguments
    /// * `input` - The input text to convert.
    /// * `dictionaries` - Slice of dictionary references, ordered by priority.
    /// * `hmm` - Whether to enable HMM mode in Jieba segmentation.
    ///
    /// # Returns
    /// A fully converted `String`, combining phrase-level and character-level replacements.
    ///
    /// # Examples
    /// ```ignore
    /// // Simplified → Traditional with phrase-first matching
    /// let opencc = OpenCC::new();
    /// let out = opencc.phrases_cut_convert(
    ///     "汉字转换示例",
    ///     &[&dict_phrases, &dict_chars],
    ///     false,
    /// );
    /// assert!(out.contains("漢字"));
    /// ```
    fn phrases_cut_convert<'a>(
        &'a self,
        input: &'a str,
        dictionaries: &'a [&DictMap],
        hmm: bool,
    ) -> String {
        let ranges = self.split_string_ranges(input, true);
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;

        if use_parallel {
            ranges
                .into_par_iter()
                .map(|range| {
                    let chunk = &input[range];
                    let mut out = String::with_capacity(chunk.len());
                    self.append_converted_chunk(chunk, dictionaries, hmm, &mut out);
                    out
                })
                .reduce(String::new, |mut a, b| {
                    a.reserve(b.len());
                    a.push_str(&b);
                    a
                })
        } else {
            let mut out = String::with_capacity(input.len() + (input.len() >> 6));
            for r in ranges {
                self.append_converted_chunk(&input[r], dictionaries, hmm, &mut out);
            }
            out
        }
    }

    /// Converts a single Jieba-produced chunk directly into the provided output buffer.
    ///
    /// This helper is shared by the serial and parallel paths in
    /// [`OpenCC::phrases_cut_convert`] so both execution modes use the same
    /// phrase lookup, delimiter fast-path, and character-fallback behavior.
    ///
    /// # Arguments
    /// * `chunk` - A non-delimiter text slice to segment and convert.
    /// * `dictionaries` - Conversion dictionaries in precedence order.
    /// * `hmm` - Whether Jieba HMM mode is enabled.
    /// * `out` - Destination buffer receiving converted output.
    #[inline(always)]
    fn append_converted_chunk(
        &self,
        chunk: &str,
        dictionaries: &[&DictMap],
        hmm: bool,
        out: &mut String,
    ) {
        let tokens = self.jieba.cut(chunk, hmm);

        'tok: for phrase in tokens {
            if phrase.is_empty() {
                continue 'tok;
            }

            let (is_single, single_char_opt, phrase_len) = Self::single_and_len(phrase);

            if is_single {
                if let Some(c) = single_char_opt {
                    if is_delimiter(c) {
                        out.push_str(phrase);
                        continue 'tok;
                    }
                }
                Self::convert_by_char(phrase, dictionaries, out);
                continue 'tok;
            }

            for dict in dictionaries {
                if !dict.has_key_len(phrase_len) {
                    continue;
                }
                if let Some(t) = dict.get(phrase) {
                    out.push_str(t);
                    continue 'tok;
                }
            }

            // Self::convert_by_char(phrase, dictionaries, out);

            let token_len = phrase.chars().count();

            if token_len < 3 {
                Self::convert_by_char(phrase, dictionaries, out);
            } else {
                Self::convert_by_fallback_fmm(phrase, dictionaries, out);
            }
        }
    }

    /// Returns whether a string is a single scalar value together with its length.
    ///
    /// This avoids re-scanning short Jieba tokens in the hot path where delimiter
    /// detection and dictionary length gating need to distinguish single-character
    /// tokens from multi-character phrases.
    ///
    /// The tuple layout is `(is_single, single_char_if_any, char_len)`.
    #[inline(always)]
    fn single_and_len(s: &str) -> (bool, Option<char>, u16) {
        let mut it = s.chars();
        match (it.next(), it.next()) {
            (None, _) => (true, None, 0),
            (Some(c), None) => (true, Some(c), 1),
            (Some(_), Some(_)) => (false, None, 2 + it.count() as u16),
        }
    }

    /// Fallback character-by-character conversion (in-place).
    ///
    /// Used when a token (phrase) is not matched in any dictionary during segmentation.
    /// Each Unicode scalar is looked up across the provided dictionaries (in priority order),
    /// and the first match wins. Output is written directly into `out` to avoid extra
    /// allocations and cloning.
    ///
    /// ## Behavior
    /// - Iterates `s.chars()` and encodes each `char` into a small stack buffer to form a `&str` key.
    /// - For each character key, searches dictionaries in order and appends the first mapped value.
    /// - If no mapping is found, appends the original character.
    ///
    /// # Arguments
    /// * `s` — Source slice to convert (typically short tokens from jieba).
    /// * `dictionaries` — Slice of dictionary references, ordered by precedence.
    /// * `out` — Output buffer to write converted text into.
    ///
    /// # Examples
    /// ```ignore
    /// // Internal helper; shown here for illustration.
    /// // In production, this is called from phrase-level conversion or st()/ts().
    /// let mut out = String::new();
    /// convert_by_char("測試", &[&dict_chars], &mut out);
    /// assert!(!out.is_empty());
    /// ```
    ///
    /// # Notes
    /// - This function is intentionally non-allocating for per-character keys (uses a stack buffer).
    /// - Keep it non-public if it is only an internal helper.
    #[inline(always)]
    fn convert_by_char(s: &str, dictionaries: &[&DictMap], out: &mut String) {
        // tiny stack buffer to avoid alloc for 1-char string creation
        // we’ll build a &str temporarily via encode_utf8
        let mut buf = [0u8; 4];

        for ch in s.chars() {
            let key = ch.encode_utf8(&mut buf); // &str for this char, no heap alloc
            let mut replaced = None;

            for dict in dictionaries {
                if dict.min_len() > 1 {
                    continue;
                }

                if let Some(v) = dict.get(key) {
                    replaced = Some(v);
                    break;
                }
            }

            match replaced {
                Some(v) => out.push_str(v),
                None => out.push(ch),
            }
        }
    }

    /// Fallback FMM conversion for an unmatched jieba token.
    ///
    /// The full token has already failed phrase lookup before entering this helper,
    /// so the first FMM window skips the full-token candidate.
    ///
    /// For short tokens, prefer `convert_by_char()` instead:
    /// - len 1: FMM is pointless
    /// - len 2: FMM degenerates to char conversion
    #[inline(always)]
    fn convert_by_fallback_fmm(s: &str, dictionaries: &[&DictMap], out: &mut String) {
        debug_assert!(s.chars().count() >= 3);

        // Longest key length that exists in any dictionary.
        let global_max_len = dictionaries.iter().map(|d| d.max_len()).max().unwrap_or(1) as usize;

        let mut start = 0usize;

        while start < s.len() {
            let rest = &s[start..];

            // Store candidate byte ends for this suffix.
            // Most OpenCC phrase lengths are small; 64 is enough for normal mask-backed keys.
            let mut ends = [0usize; 64];
            let mut count = 0usize;

            for (idx, ch) in rest.char_indices().take(global_max_len) {
                ends[count] = idx + ch.len_utf8();
                count += 1;

                if count == ends.len() {
                    break;
                }
            }

            if count == 0 {
                break;
            }

            // At start == 0, the full token was already tried and failed by caller.
            // So skip that exact full-token slice to avoid duplicate lookup.
            let skip_full_token = start == 0 && ends[count - 1] == rest.len();

            let mut replaced: Option<(&str, usize)> = None;

            for n in (0..count).rev() {
                if skip_full_token && n == count - 1 {
                    continue;
                }

                let key_len = (n + 1) as u16;
                let end = ends[n];
                let key = &rest[..end];

                for dict in dictionaries {
                    if !dict.has_key_len(key_len) {
                        continue;
                    }

                    if let Some(v) = dict.get(key) {
                        replaced = Some((v, end));
                        break;
                    }
                }

                if replaced.is_some() {
                    break;
                }
            }

            match replaced {
                Some((v, consumed)) => {
                    out.push_str(v);
                    start += consumed;
                }
                None => {
                    let ch = rest.chars().next().unwrap();
                    out.push(ch);
                    start += ch.len_utf8();
                }
            }
        }
    }

    /// Splits text into non-overlapping ranges between delimiter characters.
    ///
    /// Each `Range<usize>` corresponds to a segment bounded by or ending at a delimiter.
    ///
    /// - If `inclusive` is true, the delimiter is included at the end of each content range.
    /// - If `inclusive` is false, content segments and delimiter segments are separate ranges.
    ///
    /// Example: "Hello,World!"
    ///   Output (inclusive=true):  vec![0..6, 6..12] ("Hello,", "World!")
    ///   Output (inclusive=false): vec![0..5, 5..6, 6..11, 11..12] ("Hello", ",", "World", "!")
    pub fn split_string_ranges(&self, text: &str, inclusive: bool) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let mut current_segment_start = 0;

        // Iterate directly over char_indices for efficiency
        for (byte_idx, ch) in text.char_indices() {
            if is_delimiter(ch) {
                // Get the end byte index of the current character (delimiter)
                let ch_len = ch.len_utf8();
                let ch_end = byte_idx + ch_len;

                if inclusive {
                    // Include the delimiter at the end of the current content segment
                    ranges.push(current_segment_start..ch_end);
                } else {
                    // Exclusive: Content segment (if any)
                    if current_segment_start < byte_idx {
                        ranges.push(current_segment_start..byte_idx); // Content before delimiter
                    }
                    // Exclusive: Delimiter itself as a separate range
                    ranges.push(byte_idx..ch_end);
                }
                current_segment_start = ch_end; // Advance start for the next segment
            }
        }

        // Add the last segment if it's not empty and the string didn't end with a delimiter
        if current_segment_start < text.len() {
            ranges.push(current_segment_start..text.len());
        }

        ranges
    }

    /// Performs Jieba-based phrase segmentation over each non-delimiter chunk.
    ///
    /// This is the shared internal implementation behind the public Jieba
    /// segmentation APIs, such as [`jieba_cut()`], [`jieba_cut_for_search()`],
    /// and [`jieba_cut_all()`].
    ///
    /// The function first splits the input into non-delimiter ranges using
    /// `split_string_ranges()`, so punctuation, whitespace, and other delimiters
    /// are handled separately from lexical segmentation. Each non-delimiter chunk
    /// is then passed to the provided `cutter` function, which determines the
    /// segmentation mode.
    ///
    /// When `use_parallel` is enabled, chunk processing is parallelized with Rayon.
    /// Because the outer range iterator is indexed, the final collected output
    /// preserves the original global token order.
    ///
    /// # Type Parameters
    /// - `F` — A Jieba segmentation function that maps an input chunk to a vector
    ///   of borrowed token slices.
    ///
    /// # Arguments
    /// - `input` — The input text to segment.
    /// - `use_parallel` — Whether to process chunks in parallel using Rayon.
    /// - `cutter` — The Jieba segmentation function to apply to each chunk.
    ///
    /// # Returns
    /// A `Vec<String>` containing segmented tokens in input order.
    ///
    /// # Notes
    /// - Returned token order is deterministic, including in parallel mode.
    /// - Token slices produced by `cutter` are converted into owned [`String`] values.
    /// - Delimiters are not segmented by Jieba; they are handled by
    ///   `split_string_ranges()`.
    fn phrases_cut_impl<F>(&self, input: &str, use_parallel: bool, cutter: F) -> Vec<String>
    where
        F: for<'a> Fn(&Jieba, &'a str) -> Vec<&'a str> + Sync + Send,
    {
        let ranges = self.split_string_ranges(input, true);

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            cutter(&self.jieba, chunk).into_iter().map(str::to_owned)
        };

        if use_parallel {
            ranges
                .into_par_iter()
                .flat_map_iter(process_range)
                .collect()
        } else {
            ranges.into_iter().flat_map(process_range).collect()
        }
    }

    /// Segments input text using Jieba **accurate mode**.
    ///
    /// Accurate mode is the standard segmentation algorithm used for
    /// natural-language processing tasks. It attempts to produce the most
    /// reasonable tokenization for general text processing.
    ///
    /// The input text is first divided into non-delimiter ranges using
    /// `split_string_ranges()`. Each range is then processed by Jieba’s
    /// `cut()` function. Delimiters such as punctuation and whitespace are
    /// handled separately and are not segmented.
    ///
    /// For large inputs, segmentation may be automatically parallelized
    /// using Rayon. The final output order always matches the original
    /// input order.
    ///
    /// # Arguments
    ///
    /// * `input` — The input text to segment.
    /// * `hmm` — Whether to enable Hidden Markov Model (HMM) for unknown word detection.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing segmented tokens.
    ///
    /// # Notes
    ///
    /// - This is the **default Jieba segmentation mode**.
    /// - It balances segmentation accuracy and token count.
    /// - Parallel execution is automatically enabled for sufficiently large inputs.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    ///
    /// let tokens = opencc.jieba_cut("南京市长江大桥", true);
    ///
    /// assert!(tokens.contains(&"南京市".to_string()));
    /// ```
    pub fn jieba_cut(&self, input: &str, hmm: bool) -> Vec<String> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_cut_impl(input, use_parallel, |jieba, chunk| jieba.cut(chunk, hmm))
    }

    /// Segments input text using Jieba **search mode**.
    ///
    /// Search mode generates additional overlapping tokens to improve
    /// recall for search engines or indexing systems. Longer words are
    /// further decomposed into smaller substrings so that partial matches
    /// can be discovered during search queries.
    ///
    /// The input text is first divided into non-delimiter ranges using
    /// `split_string_ranges()`. Each range is then processed by Jieba’s
    /// `cut_for_search()` function. Delimiters such as punctuation and
    /// whitespace are handled separately and are not segmented.
    ///
    /// For large inputs, segmentation may be automatically parallelized
    /// using Rayon. The final output order always matches the original
    /// input order.
    ///
    /// # Arguments
    ///
    /// * `input` — The input text to segment.
    /// * `hmm` — Whether to enable Hidden Markov Model (HMM) for unknown word detection.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing segmented tokens suitable for search indexing.
    ///
    /// # Notes
    ///
    /// - Search mode produces **more tokens** than [`OpenCC::jieba_cut`].
    /// - Tokens may overlap due to substring generation.
    /// - Parallel execution is automatically enabled for sufficiently large inputs.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    ///
    /// let tokens = opencc.jieba_cut_for_search("南京市长江大桥", true);
    ///
    /// assert!(tokens.contains(&"南京市".to_string()));
    /// assert!(tokens.contains(&"南京".to_string()));
    /// ```
    ///
    /// # Since
    /// v0.7.3
    pub fn jieba_cut_for_search(&self, input: &str, hmm: bool) -> Vec<String> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_cut_impl(input, use_parallel, |jieba, chunk| {
            jieba.cut_for_search(chunk, hmm)
        })
    }

    /// Segments input text using Jieba **full mode**.
    ///
    /// Full mode returns all possible words that can be matched from the
    /// dictionary. This produces the largest number of tokens and is mainly
    /// useful for exhaustive text analysis.
    ///
    /// The input text is first divided into non-delimiter ranges using
    /// `split_string_ranges()`. Each range is then processed by Jieba’s
    /// `cut_all()` function. Delimiters such as punctuation and whitespace
    /// are handled separately and are not segmented.
    ///
    /// For large inputs, segmentation may be automatically parallelized
    /// using Rayon. The final output order always matches the original
    /// input order.
    ///
    /// # Arguments
    ///
    /// * `input` — The input text to segment.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing all matched tokens.
    ///
    /// # Notes
    ///
    /// - Full mode produces **significantly more tokens** than [`OpenCC::jieba_cut`].
    /// - Hidden Markov Model (HMM) is **not used** in this mode.
    /// - Parallel execution is automatically enabled for sufficiently large inputs.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    ///
    /// let tokens = opencc.jieba_cut_all("南京市长江大桥");
    ///
    /// assert!(tokens.contains(&"南京".to_string()));
    /// assert!(tokens.contains(&"南京市".to_string()));
    /// ```
    ///
    /// # Since
    /// v0.7.3
    pub fn jieba_cut_all(&self, input: &str) -> Vec<String> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_cut_impl(input, use_parallel, |jieba, chunk| jieba.cut_all(chunk))
    }

    /// Performs Jieba-based part-of-speech (POS) tagging over each non-delimiter chunk.
    ///
    /// This is the shared internal implementation behind the public POS tagging API
    /// [`jieba_tag()`].
    ///
    /// The function first splits the input into non-delimiter ranges using
    /// `split_string_ranges()`, so punctuation, whitespace, and other delimiters
    /// are handled separately from lexical analysis. Each non-delimiter chunk is
    /// then passed to the provided `tagger` function, which performs segmentation
    /// together with part-of-speech annotation.
    ///
    /// When `use_parallel` is enabled, chunk processing is parallelized with Rayon.
    /// Because the outer range iterator is indexed, the final collected output
    /// preserves the original global token order.
    ///
    /// # Type Parameters
    /// - `F` — A Jieba POS-tagging function that maps an input chunk to owned
    ///   `(word, tag)` pairs.
    ///
    /// # Arguments
    /// - `input` — The input text to tag.
    /// - `use_parallel` — Whether to process chunks in parallel using Rayon.
    /// - `tagger` — The Jieba POS-tagging function to apply to each chunk.
    ///
    /// # Returns
    /// A `Vec<(String, String)>` containing `(token, tag)` pairs in input order.
    ///
    /// # Notes
    /// - Returned token order is deterministic, including in parallel mode.
    /// - Delimiters are not tagged by Jieba; they are handled by
    ///   `split_string_ranges()`.
    fn phrases_tag_impl<F>(
        &self,
        input: &str,
        use_parallel: bool,
        tagger: F,
    ) -> Vec<(String, String)>
    where
        F: Fn(&Jieba, &str) -> Vec<(String, String)> + Sync + Send,
    {
        let ranges = self.split_string_ranges(input, true);

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            tagger(&self.jieba, chunk).into_iter()
        };

        if use_parallel {
            ranges
                .into_par_iter()
                .flat_map_iter(process_range)
                .collect()
        } else {
            ranges.into_iter().flat_map(process_range).collect()
        }
    }

    /// Tags input text using Jieba **part-of-speech (POS) tagging**.
    ///
    /// POS tagging performs segmentation and assigns a grammatical category
    /// to each token, such as noun, verb, adjective, pronoun, or English word.
    /// This is useful for downstream natural-language processing tasks such as
    /// keyword filtering, grammar analysis, readability checks, and text mining.
    ///
    /// The input text is first divided into non-delimiter ranges using
    /// `split_string_ranges()`. Each range is then processed by Jieba’s
    /// `tag()` function. Delimiters such as punctuation and whitespace are
    /// handled separately and are not tagged.
    ///
    /// For large inputs, tagging may be automatically parallelized
    /// using Rayon. The final output order always matches the original
    /// input order.
    ///
    /// # Arguments
    ///
    /// * `input` — The input text to tag.
    /// * `hmm` — Whether to enable Hidden Markov Model (HMM) for unknown word detection.
    ///
    /// # Returns
    ///
    /// A `Vec<(String, String)>` containing `(token, tag)` pairs.
    ///
    /// # Notes
    ///
    /// - This API performs both segmentation and POS annotation.
    /// - Common tags include `n` (noun), `v` (verb), `a` (adjective),
    ///   `r` (pronoun), `ns` (place name), and `eng` (English word).
    /// - Parallel execution is automatically enabled for sufficiently large inputs.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    ///
    /// let tagged = opencc.jieba_tag("我喜欢学习Rust语言", true);
    ///
    /// assert!(tagged.iter().any(|(word, tag)| word == "我" && tag == "r"));
    /// ```
    ///
    /// # Since
    /// v0.7.3
    pub fn jieba_tag(&self, input: &str, hmm: bool) -> Vec<(String, String)> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_tag_impl(input, use_parallel, |jieba, chunk| {
            jieba
                .tag(chunk, hmm)
                .into_iter()
                .map(|tag| (tag.word.to_owned(), tag.tag.to_owned()))
                .collect()
        })
    }

    /// Segments input text using Jieba and joins the result into a single string.
    ///
    /// Similar to [`OpenCC::jieba_cut`] but returns a space-separated string instead of a vector.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to be segmented.
    /// * `hmm` - Whether to enable HMM for unknown word recognition.
    ///
    /// # Returns
    ///
    /// A single `String` with segmented words joined by space.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let joined = opencc.jieba_cut_and_join("南京市长江大桥", true, " ");
    /// println!("{}", joined); // -> "南京市 长江大桥"
    /// ```
    pub fn jieba_cut_and_join(&self, input: &str, hmm: bool, delimiter: &str) -> String {
        self.jieba_cut(input, hmm).join(delimiter)
    }

    /// Converts Simplified Chinese to Traditional Chinese.
    ///
    /// This uses dictionary-based phrase mapping and segmentation via Jieba
    /// to convert Simplified Chinese (`简体中文`) into Traditional Chinese (`繁體中文`).
    ///
    /// # Arguments
    ///
    /// * `text` - The Simplified Chinese input text.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Returns
    ///
    /// A `String` containing the converted Traditional Chinese.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let s = opencc.s2t("“春眠不觉晓，处处闻啼鸟。”", true);
    /// assert_eq!(s, "「春眠不覺曉，處處聞啼鳥。」");
    /// ```
    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let result = self.phrases_cut_convert(input, &dict_refs, true);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    /// Converts Traditional Chinese to Simplified Chinese.
    ///
    /// Uses multi-stage dictionary mapping to reduce `繁體中文` into `简体中文`,
    /// optionally preserving segmentation hints.
    ///
    /// # Arguments
    ///
    /// * `text` - The Traditional Chinese input.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Returns
    ///
    /// A `String` containing the Simplified Chinese output.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let s = opencc.t2s("「春眠不覺曉，處處聞啼鳥。」", true);
    /// assert_eq!(s, "“春眠不觉晓，处处闻啼鸟。”");
    /// ```
    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let result = self.phrases_cut_convert(input, &dict_refs, true);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Simplified Chinese to Traditional Chinese (Taiwan standard).
    ///
    /// Applies additional Taiwan-specific phrase mappings after the general
    /// Simplified-to-Traditional conversion step.
    ///
    /// # Arguments
    ///
    /// * `text` - The Simplified Chinese input text.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let tw = opencc.s2tw("“春眠不觉晓，处处闻啼鸟。”", true);
    /// println!("{}", tw); // "「春眠不覺曉，處處聞啼鳥。」"
    /// ```
    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let round1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round2 = [
            &self.dictionary.tw_variants_phrases,
            &self.dictionary.tw_variants,
        ];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    /// Converts Traditional Chinese (Taiwan standard) to Simplified Chinese.
    ///
    /// Reverses Taiwan-specific phrases and maps them back to Simplified form.
    ///
    /// # Arguments
    ///
    /// * `text` - The Taiwanese Traditional Chinese input.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Returns
    ///
    /// A `String` in Simplified Chinese.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let simp = opencc.tw2s("「春眠不覺曉，處處聞啼鳥。」", true);
    /// println!("{}", simp); // "“春眠不觉晓，处处闻啼鸟。”"
    /// ```
    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let round1 = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let round2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Simplified Chinese to Traditional Chinese (Taiwan) with punctuation.
    ///
    /// Performs two dictionary-conversion rounds:
    ///
    /// 1. Simplified-to-Traditional phrases and characters.
    /// 2. Taiwan phrases, variant phrases, and character variants together,
    ///    in that priority order.
    ///
    /// Punctuation marks are optionally converted after both dictionary rounds,
    /// including quote styles (`“”` → `「」`).
    ///
    /// # Arguments
    ///
    /// * `text` - The Simplified Chinese input.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let result = opencc.s2twp("“你好，世界”", true);
    /// assert_eq!(result.contains("「你好，世界」"), true);
    /// ```
    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let round1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round2 = [
            &self.dictionary.tw_phrases,
            &self.dictionary.tw_variants_phrases,
            &self.dictionary.tw_variants,
        ];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    /// Converts Taiwanese Traditional Chinese to Simplified Chinese with punctuation.
    ///
    /// This method includes punctuation transformation (e.g., `「」` → `“”`)
    /// in addition to textual content replacement.
    ///
    /// # Arguments
    ///
    /// * `text` - The Traditional Chinese input.
    /// * `hmm` - Whether to enable HMM-based segmentation.
    ///
    /// # Returns
    ///
    /// A fully simplified and punctuated version.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let result = opencc.tw2sp("「春眠不覺曉，處處聞啼鳥。」", true);
    /// assert!(result.contains("“春眠不觉晓，处处闻啼鸟。”"));
    /// ```
    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let round1 = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let round2 = [&self.dictionary.tw_phrases_rev];
        let round3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        let result = self.convert_rounds(input, &[&round1, &round2, &round3], true);

        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Simplified Chinese to Traditional Chinese (Hong Kong standard).
    ///
    /// This adds phrase mapping specific to the Hong Kong locale after a
    /// general Simplified-to-Traditional conversion step.
    ///
    /// # Arguments
    ///
    /// * `text` - Simplified Chinese text.
    /// * `hmm` - Whether to enable HMM segmentation.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let hk = opencc.s2hk("“春眠不觉晓，处处闻啼鸟。”", true);
    /// println!("{}", hk); // "「春眠不覺曉，處處聞啼鳥。」"
    /// ```
    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let round1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round2 = [
            &self.dictionary.hk_variants_phrases,
            &self.dictionary.hk_variants,
        ];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    /// Converts Simplified Chinese to Hong Kong Traditional with phrase
    /// preferences (S → T → HKP).
    ///
    /// Round 1 applies the standard Simplified-to-Traditional dictionaries.
    /// Round 2 applies Hong Kong phrases, variant phrases, and character
    /// variants together in priority order.
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.s2hkp("鼠标", false), "滑鼠");
    /// ```
    pub fn s2hkp(&self, input: &str, punctuation: bool) -> String {
        let round1 = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let round2 = [
            &self.dictionary.hk_phrases,
            &self.dictionary.hk_variants_phrases,
            &self.dictionary.hk_variants,
        ];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    /// Converts Traditional Chinese to Simplified Chinese (Hong Kong standard).
    ///
    /// This adds phrase mapping specific to the Hong Kong locale after a
    /// general Simplified-to-Traditional conversion step.
    ///
    /// # Arguments
    ///
    /// * `text` - Simplified Chinese text.
    /// * `hmm` - Whether to enable HMM segmentation.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let hk = opencc.hk2s("「春眠不覺曉，處處聞啼鳥。」", true);
    /// println!("{}", hk); // "「春眠不覺曉，處處聞啼鳥。」"
    /// ```
    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let round1 = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let round2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Hong Kong Traditional text with phrase preferences to
    /// Simplified Chinese (HKP → T → S).
    ///
    /// Round 1 normalizes Hong Kong phrases, reverse variant phrases, and
    /// character variants together. Round 2 applies the standard
    /// Traditional-to-Simplified dictionaries.
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.hk2sp("滑鼠", false), "鼠标");
    /// ```
    pub fn hk2sp(&self, input: &str, punctuation: bool) -> String {
        let round1 = [
            &self.dictionary.hk_phrases_rev,
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let round2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];

        let result = self.convert_rounds(input, &[&round1, &round2], true);

        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Traditional Chinese (T) text to **Taiwan Traditional Chinese (T→Tw)**.
    ///
    /// This corresponds to the OpenCC configuration **`t2tw`**, applying
    /// Taiwan-specific character variants.
    ///
    /// The conversion performs:
    /// - Phrase-level segmentation via Jieba
    /// - Dictionary-based replacements using **`tw_variants_phrases`** before **`tw_variants`**
    /// - Optional punctuation conversion (enabled)
    ///
    /// # Arguments
    /// - `input` — UTF-8 Traditional Chinese text.
    ///
    /// # Returns
    /// A `String` containing the converted Taiwan Traditional Chinese output.
    ///
    /// # Example
    /// ```ignore
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let out = opencc.t2tw("繁體字");
    /// ```
    pub fn t2tw(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_phrases,
            &self.dictionary.tw_variants,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Traditional Chinese (T) text to **Taiwan Traditional Chinese with
    /// phrase-level idioms (T→Twp)**.
    ///
    /// This corresponds to the OpenCC configuration **`t2twp`**. It segments the
    /// input once and applies these dictionaries in precedence order:
    ///
    /// 1. `tw_phrases`
    /// 2. `tw_variants_phrases`
    /// 3. `tw_variants`
    ///
    /// The first matching dictionary wins. A replacement is emitted directly and
    /// is not passed through the remaining dictionaries, so dictionary precedence
    /// remains deterministic without a second conversion round. Punctuation is
    /// preserved.
    ///
    /// # Arguments
    /// - `input` — UTF-8 Traditional Chinese text.
    ///
    /// # Returns
    /// A `String` containing the fully converted Taiwan Traditional Chinese result
    /// with Taiwan-specific idioms and variants.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.t2twp("鼠標"), "滑鼠");
    /// ```
    pub fn t2twp(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_phrases,
            &self.dictionary.tw_variants_phrases,
            &self.dictionary.tw_variants,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts **Taiwan Traditional Chinese (Tw)** text to **Standard Traditional
    /// Chinese (T)**.
    ///
    /// This corresponds to the OpenCC configuration **`tw2t`**.
    /// It removes Taiwan-specific character variants and idioms, normalizing the
    /// text back to standard Traditional Chinese.
    ///
    /// This function performs:
    /// - Phrase-level segmentation using Jieba
    /// - A single-round dictionary replacement using:
    ///   - `tw_variants_rev` (reverse character variants)
    ///   - `tw_variants_rev_phrases` (reverse phrase-level variants)
    /// - Punctuation conversion is enabled
    ///
    /// # Arguments
    /// - `input` — UTF-8 Taiwan Traditional Chinese text.
    ///
    /// # Returns
    /// A `String` containing the normalized Traditional Chinese output.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let text = "裡面"; // Taiwan variant
    /// let out = opencc.tw2t(text);
    /// assert_eq!(out, "裡面"); // Standard Traditional
    /// ```
    pub fn tw2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts **Taiwan Traditional Chinese (Tw)** to **Standard Traditional Chinese
    /// with phrase refinement (Tw→Tp)**.
    ///
    /// This corresponds to the OpenCC configuration **`tw2tp`**. It segments the
    /// input once and applies these dictionaries in precedence order:
    ///
    /// 1. `tw_variants_rev`
    /// 2. `tw_variants_rev_phrases`
    /// 3. `tw_phrases_rev`
    ///
    /// The first matching dictionary wins. A replacement is emitted directly and
    /// is not passed through the remaining dictionaries, avoiding a second
    /// segmentation and conversion round. Punctuation is preserved.
    ///
    /// # Arguments
    /// - `input` — UTF-8 Taiwan Traditional Chinese text.
    ///
    /// # Returns
    /// A `String` containing fully normalized Traditional Chinese with enhanced
    /// phrase-level corrections.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.tw2tp("滑鼠"), "鼠標");
    /// ```
    pub fn tw2tp(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
            &self.dictionary.tw_phrases_rev,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Standard Traditional Chinese (T) text to **Hong Kong Traditional
    /// Chinese (T→HK)**.
    ///
    /// This corresponds to the OpenCC configuration **`t2hk`**, applying
    /// Hong-Kong–specific character variants and phrase preferences.
    ///
    /// Phrase-level segmentation is used internally, and punctuation conversion is
    /// enabled.
    pub fn t2hk(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_phrases,
            &self.dictionary.hk_variants,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Traditional Chinese (T) to **Hong Kong Traditional Chinese with
    /// phrase preferences (T→HKP)**.
    ///
    /// This corresponds to the **`t2hkp`** configuration. It segments the input
    /// once and applies these dictionaries in precedence order:
    ///
    /// 1. `hk_phrases`
    /// 2. `hk_variants_phrases`
    /// 3. `hk_variants`
    ///
    /// The first matching dictionary wins. A replacement is emitted directly and
    /// is not passed through the remaining dictionaries, preserving deterministic
    /// dictionary precedence without a second conversion round. Punctuation is
    /// preserved.
    ///
    /// # Arguments
    ///
    /// * `input` - UTF-8 Traditional Chinese text.
    ///
    /// # Returns
    ///
    /// Hong Kong Traditional Chinese using regional phrases and variants.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.t2hkp("鼠標"), "滑鼠");
    /// ```
    pub fn t2hkp(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.hk_phrases,
            &self.dictionary.hk_variants_phrases,
            &self.dictionary.hk_variants,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Hong Kong Traditional Chinese text back to **Standard Traditional
    /// Chinese (HK→T)**.
    ///
    /// This corresponds to the OpenCC configuration **`hk2t`**, applying the reverse
    /// transformation using:
    /// - Hong-Kong–specific reverse phrase mappings (`hk_variants_rev_phrases`)
    /// - Reverse character-level mappings (`hk_variants_rev`)
    ///
    /// Phrase segmentation is applied before replacement, with punctuation
    /// conversion enabled.
    pub fn hk2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Hong Kong Traditional Chinese (HK) to **Standard Traditional
    /// Chinese with phrase normalization (HK→TP)**.
    ///
    /// This corresponds to the **`hk2tp`** configuration. It segments the input
    /// once and applies these dictionaries in precedence order:
    ///
    /// 1. `hk_variants_rev_phrases`
    /// 2. `hk_variants_rev`
    /// 3. `hk_phrases_rev`
    ///
    /// The first matching dictionary wins. A replacement is emitted directly and
    /// is not passed through the remaining dictionaries, avoiding a second
    /// segmentation and conversion round. Punctuation is preserved.
    ///
    /// # Arguments
    ///
    /// * `input` - UTF-8 Hong Kong Traditional Chinese text.
    ///
    /// # Returns
    ///
    /// Standard Traditional Chinese with Hong Kong phrases and variants normalized.
    ///
    /// # Example
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.hk2tp("滑鼠"), "鼠標");
    /// ```
    pub fn hk2tp(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
            &self.dictionary.hk_phrases_rev,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    /// Converts Traditional Chinese (T) text to **Japanese Shinjitai (T→JP)**.
    ///
    /// This corresponds to the OpenCC configuration **`t2jp`**, applying the
    /// Japanese character-variant set (“Shinjitai”).
    ///
    /// Phrase-level segmentation is performed, and punctuation conversion is
    /// enabled.
    /// Note that this is **not a Japanese translation**—only character forms are
    /// converted.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let out = opencc.t2jp("體育");    // 體 → 体
    /// assert_eq!(out, "体育");         // Standard Japanese Shinjitai form
    /// ```
    pub fn t2jp(&self, input: &str) -> String {
        let dictionary = if self.dictionary.schema_version < 3
            && self.dictionary.jps_characters_rev.is_empty()
        {
            &self.dictionary.legacy_jp_variants
        } else {
            &self.dictionary.jps_characters_rev
        };
        self.phrases_cut_convert(input, &[dictionary], true)
    }

    /// Converts **Japanese Shinjitai (JP)** text back to **Traditional Chinese (T)**.
    ///
    /// This corresponds to the OpenCC configuration **`jp2t`**, performing a
    /// reverse transformation of:
    ///
    /// - Japanese phrase-level variants (`jps_phrases`)
    /// - Japanese character simplifications (`jps_characters`)
    ///
    /// Phrase-level segmentation is applied, and punctuation conversion is enabled.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    ///
    /// // Common Shinjitai → Traditional:
    /// assert_eq!(opencc.jp2t("教育"), "教育");       // unchanged (identical)
    /// assert_eq!(opencc.jp2t("体力"), "體力");       // 体 → 體
    /// assert_eq!(opencc.jp2t("図書"), "圖書");       // 図 → 圖
    /// ```
    pub fn jp2t(&self, input: &str) -> String {
        if self.dictionary.schema_version < 3 && !self.dictionary.legacy_jp_variants_rev.is_empty()
        {
            let dict_refs = [
                &self.dictionary.jps_phrases,
                &self.dictionary.jps_characters,
                &self.dictionary.legacy_jp_variants_rev,
            ];
            self.phrases_cut_convert(input, &dict_refs, true)
        } else {
            let dict_refs = [
                &self.dictionary.jps_phrases,
                &self.dictionary.jps_characters,
            ];
            self.phrases_cut_convert(input, &dict_refs, true)
        }
    }

    /// Applies multiple dictionary-conversion rounds sequentially.
    ///
    /// This helper implements the **multi-stage dictionary pipeline** used by
    /// several OpenCC configurations (such as `s2twp`, `tw2sp`, `hk2s`, etc.).
    ///
    /// Each round is a slice of [`DictMap`] dictionaries applied in priority order.
    /// The output of one round becomes the input to the next.
    ///
    /// Conceptually the pipeline looks like:
    ///
    /// ```text
    /// input
    ///   ↓ round1 (phrase dictionaries)
    ///   ↓ round2 (variant dictionaries)
    ///   ↓ round3 (final normalization)
    /// output
    /// ```
    ///
    /// Internally this simply loops through each round and calls
    /// [`phrases_cut_convert`] with the corresponding dictionary slice.
    ///
    /// # Arguments
    ///
    /// * `input` - The source UTF-8 text.
    /// * `rounds` - Ordered sequence of dictionary rounds.
    /// * `hmm` - Whether to enable Jieba HMM segmentation.
    ///
    /// # Returns
    ///
    /// A newly allocated `String` containing the fully converted text.
    ///
    /// # Notes
    ///
    /// - This function is an **internal helper** used to simplify conversion
    ///   pipelines and reduce nested `phrases_cut_convert` calls.
    /// - Performance is identical to the nested version since each round
    ///   still performs a single conversion pass.
    ///
    /// [`DictMap`]: DictMap
    #[inline]
    fn convert_rounds(&self, input: &str, rounds: DictRounds<'_>, hmm: bool) -> String {
        if rounds.is_empty() {
            return input.to_owned();
        }

        let mut current = input.to_owned();
        for round in rounds {
            current = self.phrases_cut_convert(&current, round, hmm);
        }
        current
    }

    /// Performs **fast character-level Simplified → Traditional** Chinese conversion.
    ///
    /// This corresponds to OpenCC’s **`st`** character-variant mapping and uses
    /// **only** the `st_characters` dictionary.
    ///
    /// Unlike phrase-level conversions (e.g., `s2t`, `s2tw`), this function:
    /// - **does not** use Jieba segmentation
    /// - **does not** perform phrase matching
    /// - applies **single-character substitutions only**
    ///
    /// This makes it ideal for:
    /// - punctuation or symbol normalization
    /// - environments requiring minimal overhead
    /// - preprocessing before higher-level conversion
    ///
    /// # Example
    /// ```ignore
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.st("后"), "後"); // Character-level only
    /// ```
    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let mut output = String::with_capacity(input.len());
        Self::convert_by_char(input, &dict_refs, &mut output);
        output
    }

    /// Performs **fast character-level Traditional → Simplified** Chinese conversion.
    ///
    /// This corresponds to OpenCC’s **`ts`** character-variant mapping and uses
    /// **only** the `ts_characters` dictionary.
    ///
    /// Unlike phrase-level conversions (e.g., `t2s`, `tw2s`), this function:
    /// - **does not** use Jieba segmentation
    /// - **does not** perform phrase matching
    /// - applies **single-character substitutions only**
    ///
    /// This makes it ideal for:
    /// - punctuation or symbol normalization
    /// - environments requiring minimal overhead
    /// - preprocessing before higher-level conversion
    ///
    /// # Example
    /// ```ignore
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.ts("後"), "后"); // Character-level only
    /// ```
    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let mut output = String::with_capacity(input.len());
        Self::convert_by_char(input, &dict_refs, &mut output);
        output
    }

    /// Converts Chinese text between different variants using a configuration string.
    ///
    /// This is the **generic entry point** for OpenCC-style conversions. It supports
    /// conversion between Simplified, Traditional, Taiwan, Hong Kong, and Japanese
    /// character variants.
    ///
    /// Internally this function parses `config` into [`OpenccConfig`] and dispatches
    /// to the corresponding conversion pipeline.
    ///
    /// For Rust callers, using [`convert_with_config`] with the strongly-typed
    /// [`OpenccConfig`] enum is recommended to avoid string parsing.
    ///
    /// # Arguments
    ///
    /// * `input` - The UTF-8 text to convert.
    /// * `config` - Conversion configuration (case-insensitive). Supported values:
    ///
    ///   - `"s2t"`   — Simplified → Traditional
    ///   - `"s2tw"`  — Simplified → Taiwan Traditional
    ///   - `"s2twp"` — Simplified → Taiwan Traditional (phrase-level refinement)
    ///   - `"s2hk"`  — Simplified → Hong Kong Traditional
    ///   - `"s2hkp"` — Simplified → Hong Kong Traditional (phrase preferences)
    ///   - `"t2s"`   — Traditional → Simplified
    ///   - `"t2tw"`  — Traditional → Taiwan Traditional
    ///   - `"t2twp"` — Traditional → Taiwan Traditional (phrase-level refinement)
    ///   - `"t2hk"`  — Traditional → Hong Kong Traditional
    ///   - `"t2hkp"` — Traditional → Hong Kong Traditional (phrase preferences)
    ///   - `"tw2s"`  — Taiwan Traditional → Simplified
    ///   - `"tw2sp"` — Taiwan Traditional → Simplified (phrase-level refinement)
    ///   - `"tw2t"`  — Taiwan Traditional → Standard Traditional
    ///   - `"tw2tp"` — Taiwan Traditional → Standard Traditional (phrase refinement)
    ///   - `"hk2s"`  — Hong Kong Traditional → Simplified
    ///   - `"hk2sp"` — Hong Kong Traditional → Simplified (phrase preferences)
    ///   - `"hk2t"`  — Hong Kong Traditional → Standard Traditional
    ///   - `"hk2tp"` — Hong Kong Traditional → Standard Traditional (phrase normalization)
    ///   - `"jp2t"`  — Japanese Shinjitai → Traditional Chinese
    ///   - `"t2jp"`  — Traditional Chinese → Japanese Shinjitai
    ///
    /// * `punctuation` - Whether to convert punctuation marks (e.g. `“”` ↔ `「」`)
    ///   when applicable.
    ///   Some configurations ignore this flag if punctuation normalization does
    ///   not apply to that conversion pipeline.
    ///
    /// # Returns
    ///
    /// A newly allocated `String` containing the converted text.
    ///
    /// If `config` is invalid, the function returns a string in the form:
    ///
    /// ```text
    /// Invalid config: <config>
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// let opencc = OpenCC::new();
    ///
    /// let traditional = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "s2t", true);
    /// let taiwanese = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "s2tw", true);
    ///
    /// let invalid = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "unknown", true);
    /// assert_eq!(invalid, "Invalid config: unknown");
    /// ```
    ///
    /// [`convert_with_config`]: OpenCC::convert_with_config
    /// [`OpenccConfig`]: OpenccConfig
    pub fn convert(&self, input: &str, config: &str, punctuation: bool) -> String {
        match OpenccConfig::try_from(config) {
            Ok(cfg) => self.convert_with_config(input, cfg, punctuation),
            Err(_) => {
                format!("Invalid config: {}", config)
            }
        }
    }

    /// Converts Chinese text using a strongly-typed [`OpenccConfig`].
    ///
    /// This method avoids string parsing and is the recommended API for Rust callers.
    /// It also maps cleanly to the C FFI numeric config (`opencc_config_t`).
    ///
    /// # Arguments
    ///
    /// * `input` - UTF-8 text to convert.
    /// * `config_id` - Conversion configuration.
    /// * `punctuation` - Whether to apply punctuation conversion where supported.
    ///   For some configs, this flag is **ignored** (see [`OpenccConfig`] table).
    ///
    /// # Example
    ///
    /// ```rust
    /// use opencc_jieba_rs::{OpenCC, OpenccConfig};
    ///
    /// let converter = OpenCC::new();
    /// let out = converter.convert_with_config("汉字转换测试", OpenccConfig::S2t, false);
    /// assert_eq!(out, "漢字轉換測試");
    /// ```
    pub fn convert_with_config(
        &self,
        input: &str,
        config_id: OpenccConfig,
        punctuation: bool,
    ) -> String {
        match config_id {
            OpenccConfig::S2t => self.s2t(input, punctuation),
            OpenccConfig::S2tw => self.s2tw(input, punctuation),
            OpenccConfig::S2twp => self.s2twp(input, punctuation),
            OpenccConfig::S2hk => self.s2hk(input, punctuation),
            OpenccConfig::S2hkp => self.s2hkp(input, punctuation),
            OpenccConfig::T2s => self.t2s(input, punctuation),
            OpenccConfig::T2tw => self.t2tw(input),
            OpenccConfig::T2twp => self.t2twp(input),
            OpenccConfig::T2hk => self.t2hk(input),
            OpenccConfig::T2hkp => self.t2hkp(input),
            OpenccConfig::Tw2s => self.tw2s(input, punctuation),
            OpenccConfig::Tw2sp => self.tw2sp(input, punctuation),
            OpenccConfig::Tw2t => self.tw2t(input),
            OpenccConfig::Tw2tp => self.tw2tp(input),
            OpenccConfig::Hk2s => self.hk2s(input, punctuation),
            OpenccConfig::Hk2sp => self.hk2sp(input, punctuation),
            OpenccConfig::Hk2t => self.hk2t(input),
            OpenccConfig::Hk2tp => self.hk2tp(input),
            OpenccConfig::Jp2t => self.jp2t(input),
            OpenccConfig::T2jp => self.t2jp(input),
        }
    }

    /// Checks the type of Chinese text (Simplified, Traditional, or Other).
    ///
    /// This helper function analyzes the input string and determines whether it is written in Simplified Chinese,
    /// Traditional Chinese, or neither. It does so by stripping non-Chinese characters, truncating to a maximum
    /// of 200 bytes (without splitting UTF-8 characters), and comparing the result to its converted forms.
    ///
    /// # Arguments
    ///
    /// * `input` - The input string to check.
    ///
    /// # Returns
    ///
    /// An `i32` code indicating the type of Chinese text:
    /// - `2`: Simplified Chinese
    /// - `1`: Traditional Chinese
    /// - `0`: Other or undetermined
    ///
    /// # Examples
    ///
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// assert_eq!(opencc.zho_check("“春眠不觉晓，处处闻啼鸟。”"), 2);
    /// assert_eq!(opencc.zho_check("「春眠不覺曉，處處聞啼鳥。」"), 1);
    /// assert_eq!(opencc.zho_check("Hello World!"), 0);
    /// ```
    pub fn zho_check(&self, input: &str) -> i32 {
        if input.is_empty() {
            return 0;
        }
        let check_length = find_max_utf8_length(input, 1000);
        let _strip_text = strip_regex().replace_all(&input[..check_length], "");
        let max_bytes = find_max_utf8_length(_strip_text.as_ref(), 200);
        let strip_text = &_strip_text[..max_bytes];
        let code;
        if strip_text != &self.ts(strip_text) {
            code = 1;
        } else {
            if strip_text != &self.st(strip_text) {
                code = 2;
            } else {
                code = 0;
            }
        }
        code
    }

    /// Converts Chinese punctuation marks between Simplified and Traditional variants.
    ///
    /// This helper function replaces punctuation marks in the input text according to the specified configuration.
    /// If `config` starts with `'s'`, it converts Simplified punctuation to Traditional; otherwise, it converts
    /// Traditional punctuation to Simplified.
    ///
    /// # Arguments
    ///
    /// * `text` - The input string whose punctuation will be converted.
    /// * `config` - The conversion configuration (`"s"` for Simplified to Traditional, otherwise Traditional to Simplified).
    ///
    /// # Returns
    ///
    /// A `String` with punctuation marks converted according to the specified variant.
    fn convert_punctuation(text: &str, config: &str) -> String {
        let mut out = String::with_capacity(text.len());
        if config.starts_with('s') {
            for ch in text.chars() {
                out.push(match ch {
                    '“' => '「',
                    '”' => '」',
                    '‘' => '『',
                    '’' => '』',
                    _ => ch,
                });
            }
        } else {
            for ch in text.chars() {
                out.push(match ch {
                    '「' => '“',
                    '」' => '”',
                    '『' => '‘',
                    '』' => '’',
                    _ => ch,
                });
            }
        }
        out
    }

    /// Extracts top keywords using the TextRank algorithm.
    ///
    /// TextRank is a graph-based algorithm that ranks words based on co-occurrence.
    /// This method segments the input, builds a co-occurrence graph, and returns the
    /// top N keywords with the highest scores.
    ///
    /// # Arguments
    ///
    /// * `text` - Input text to extract keywords from.
    /// * `topk` - Maximum number of keywords to return.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of keywords sorted by importance.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let keywords = opencc.keyword_extract_textrank("自然语言处理和机器学习", 5);
    /// println!("{:?}", keywords);
    /// ```
    pub fn keyword_extract_textrank(&self, input: &str, top_k: usize) -> Vec<String> {
        keyword_extract_internal(&self.jieba, input, top_k, KeywordMethod::TextRank, None)
    }

    /// Extracts top keywords using the TextRank algorithm with optional POS filtering.
    ///
    /// This method performs word segmentation and applies the TextRank algorithm
    /// to rank keywords based on co-occurrence relationships. It optionally filters
    /// candidate words by part-of-speech (POS) tags before ranking.
    ///
    /// # Arguments
    ///
    /// * `input` - Input text to extract keywords from.
    /// * `top_k` - Maximum number of keywords to return.
    /// * `allowed_pos` - Optional slice of POS tags used to filter candidates.
    ///   For example: `Some(&["n", "nr", "ns", "v"])`.
    ///   - `None` means no POS filtering (all words are considered).
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the top keywords sorted by importance.
    ///
    /// # Example
    /// ```
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// let opencc = OpenCC::new();
    ///
    /// let keywords = opencc.keyword_extract_textrank_pos(
    ///     "自然语言处理和机器学习",
    ///     5,
    ///     Some(&["n", "nr", "v"]),
    /// );
    ///
    /// println!("{:?}", keywords);
    /// ```
    ///
    /// # Notes
    ///
    /// - POS tags follow the Jieba tagging scheme.
    /// - Common useful tags include:
    ///   - `"n"`  (noun)
    ///   - `"nr"` (person name)
    ///   - `"ns"` (place name)
    ///   - `"nt"` (organization)
    ///   - `"v"`  (verb)
    ///
    /// - For best results, restrict to content words (e.g., nouns and verbs).
    ///
    /// # See also
    ///
    /// - [`keyword_extract_textrank`](Self::keyword_extract_textrank)
    /// - [`keyword_weight_textrank`](Self::keyword_weight_textrank)
    /// - [`keyword_weight_textrank_pos`](Self::keyword_weight_textrank_pos)
    ///
    /// # Since
    /// v0.7.4
    pub fn keyword_extract_textrank_pos(
        &self,
        input: &str,
        top_k: usize,
        allowed_pos: Option<&[&str]>,
    ) -> Vec<String> {
        keyword_extract_internal(
            &self.jieba,
            input,
            top_k,
            KeywordMethod::TextRank,
            allowed_pos,
        )
    }

    /// Returns weighted keywords using the TextRank algorithm.
    ///
    /// This method segments and ranks words based on TextRank and returns
    /// a list of keyword-weight pairs as [`Keyword`] objects.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to analyze.
    /// * `topk` - Number of top keywords to return.
    ///
    /// # Returns
    ///
    /// A `Vec<Keyword>` — each keyword has `.keyword` and `.weight` fields.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let weighted = opencc.keyword_weight_textrank("自然语言处理和机器学习", 5);
    /// for kw in weighted {
    ///     println!("{}: {}", kw.keyword, kw.weight);
    /// }
    /// ```
    ///
    /// [`Keyword`]: https://docs.rs/jieba-rs/latest/jieba_rs/struct.Keyword.html
    pub fn keyword_weight_textrank(&self, input: &str, top_k: usize) -> Vec<Keyword> {
        keyword::keyword_weight_textrank_internal(&self.jieba, input, top_k, None)
    }

    /// Returns weighted keywords using the TextRank algorithm with optional POS filtering.
    ///
    /// This method behaves the same as [`Self::keyword_weight_textrank`], but allows filtering
    /// by part-of-speech (POS) tags.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to analyze.
    /// * `topk` - Number of top keywords to return.
    /// * `allowed_pos` - Optional slice of POS tags (e.g. `Some(&["n", "nr", "v"])`).
    ///
    /// # Returns
    ///
    /// A `Vec<Keyword>` — each keyword has `.keyword` and `.weight` fields.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let weighted = opencc.keyword_weight_textrank_pos(
    ///     "自然语言处理和机器学习",
    ///     5,
    ///     Some(&["n", "nr", "v"]),
    /// );
    /// for kw in weighted {
    ///     println!("{}: {}", kw.keyword, kw.weight);
    /// }
    /// ```
    ///
    /// # Since
    /// v0.7.4
    pub fn keyword_weight_textrank_pos(
        &self,
        input: &str,
        top_k: usize,
        allowed_pos: Option<&[&str]>,
    ) -> Vec<Keyword> {
        keyword::keyword_weight_textrank_internal(&self.jieba, input, top_k, allowed_pos)
    }

    /// Extracts top keywords using the TF-IDF algorithm.
    ///
    /// This method uses Jieba's internal IDF table and segmentation to rank keywords
    /// based on term frequency-inverse document frequency.
    ///
    /// # Arguments
    ///
    /// * `text` - Input text to analyze.
    /// * `topk` - Maximum number of keywords to return.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of top-ranked keywords.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let keywords = opencc.keyword_extract_tfidf("深度学习正在改变人工智能", 5);
    /// println!("{:?}", keywords);
    /// ```
    pub fn keyword_extract_tfidf(&self, input: &str, top_k: usize) -> Vec<String> {
        keyword_extract_internal(&self.jieba, input, top_k, KeywordMethod::TfIdf, None)
    }

    /// Extracts top keywords using the TF-IDF algorithm with optional POS filtering.
    ///
    /// This method performs word segmentation and ranks keywords based on their
    /// TF-IDF (Term Frequency–Inverse Document Frequency) scores. It optionally
    /// filters candidate words by part-of-speech (POS) tags before ranking.
    ///
    /// # Arguments
    ///
    /// * `input` - Input text to extract keywords from.
    /// * `top_k` - Maximum number of keywords to return.
    /// * `allowed_pos` - Optional slice of POS tags used to filter candidates.
    ///   For example: `Some(&["n", "nr", "ns", "v"])`.
    ///   - `None` means no POS filtering (all words are considered).
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing the top keywords sorted by importance.
    ///
    /// # Example
    /// ```
    /// use opencc_jieba_rs::OpenCC;
    ///
    /// let opencc = OpenCC::new();
    ///
    /// let keywords = opencc.keyword_extract_tfidf_pos(
    ///     "深度学习正在改变人工智能",
    ///     5,
    ///     Some(&["n", "nr", "v"]),
    /// );
    ///
    /// println!("{:?}", keywords);
    /// ```
    ///
    /// # Notes
    ///
    /// - POS tags follow the Jieba tagging scheme.
    /// - Common useful tags include:
    ///   - `"n"`  (noun)
    ///   - `"nr"` (person name)
    ///   - `"ns"` (place name)
    ///   - `"nt"` (organization)
    ///   - `"v"`  (verb)
    ///
    /// - TF-IDF favors words that are frequent in the input text but less common
    ///   across the corpus, making it suitable for keyword extraction in
    ///   domain-specific or technical text.
    ///
    /// # See also
    ///
    /// - [`keyword_extract_tfidf`](Self::keyword_extract_tfidf)
    /// - [`keyword_weight_tfidf`](Self::keyword_weight_tfidf)
    /// - [`keyword_weight_tfidf_pos`](Self::keyword_weight_tfidf_pos)
    ///
    /// # Since
    /// v0.7.4
    pub fn keyword_extract_tfidf_pos(
        &self,
        input: &str,
        top_k: usize,
        allowed_pos: Option<&[&str]>,
    ) -> Vec<String> {
        keyword_extract_internal(&self.jieba, input, top_k, KeywordMethod::TfIdf, allowed_pos)
    }

    /// Returns weighted keywords using the TF-IDF algorithm.
    ///
    /// This method segments the input and ranks keywords by TF-IDF weight, returning
    /// structured keyword objects with their scores.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to analyze.
    /// * `topk` - Number of top keywords to return.
    ///
    /// # Returns
    ///
    /// A `Vec<jieba_rs::Keyword>`, each with:
    /// - `keyword`: The extracted word.
    /// - `weight`: The TF-IDF score representing word importance.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let weighted = opencc.keyword_weight_tfidf("深度学习正在改变人工智能", 5);
    /// for kw in weighted {
    ///     println!("{}: {}", kw.keyword, kw.weight);
    /// }
    /// ```
    ///
    /// [`Keyword`]: https://docs.rs/jieba-rs/latest/jieba_rs/struct.Keyword.html
    pub fn keyword_weight_tfidf(&self, input: &str, top_k: usize) -> Vec<Keyword> {
        keyword::keyword_weight_tfidf_internal(&self.jieba, input, top_k, None)
    }

    /// Returns weighted keywords using the TF-IDF algorithm with optional POS filtering.
    ///
    /// This method behaves the same as [`Self::keyword_weight_tfidf`], but allows filtering
    /// by part-of-speech (POS) tags.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to analyze.
    /// * `topk` - Number of top keywords to return.
    /// * `allowed_pos` - Optional slice of POS tags (e.g. `Some(&["n", "nr", "v"])`).
    ///
    /// # Returns
    ///
    /// A `Vec<jieba_rs::Keyword>`, each with:
    /// - `keyword`: The extracted word.
    /// - `weight`: The TF-IDF score representing word importance.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let weighted = opencc.keyword_weight_tfidf_pos(
    ///     "深度学习正在改变人工智能",
    ///     5,
    ///     Some(&["n", "nr", "v"]),
    /// );
    /// for kw in weighted {
    ///     println!("{}: {}", kw.keyword, kw.weight);
    /// }
    /// ```
    ///
    /// # Since
    /// v0.7.4
    pub fn keyword_weight_tfidf_pos(
        &self,
        input: &str,
        top_k: usize,
        allowed_pos: Option<&[&str]>,
    ) -> Vec<Keyword> {
        keyword::keyword_weight_tfidf_internal(&self.jieba, input, top_k, allowed_pos)
    }
}

/// Provides a default [`OpenCC`] instance.
///
/// This is equivalent to calling [`OpenCC::new`]. It allows `OpenCC`
/// to integrate with common Rust patterns where types implement
/// [`Default`], such as:
///
/// - `OpenCC::default()`
/// - `#[derive(Default)]` in structs containing `OpenCC`
/// - generic APIs requiring `T: Default`
///
/// # Examples
///
/// ```
/// use opencc_jieba_rs::OpenCC;
///
/// let opencc = OpenCC::default();
/// let result = opencc.convert("汉字", "s2t", false);
///
/// assert_eq!(result, "漢字");
/// ```
///
/// [`Default`]: Default
impl Default for OpenCC {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn dict(entries: &[(&str, &str)]) -> DictMap {
        let mut map = DictMap::default();
        for (from, to) in entries {
            map.insert_with_len(
                (*from).to_string(),
                (*to).to_string(),
                from.chars().count() as u16,
            );
        }
        map
    }

    fn opencc_with_dictionary(dictionary: Dictionary, words: &[&str]) -> OpenCC {
        let mut jieba = Jieba::new();
        for word in words {
            jieba.add_word(word, Some(100_000), Some("n"));
        }

        OpenCC {
            jieba: Arc::new(jieba),
            dictionary,
        }
    }

    #[test]
    fn tw_forward_variants_apply_phrases_before_characters() {
        let mut dictionary = Dictionary::default();
        dictionary.tw_variants_phrases = dict(&[("純喫茶", "純喫茶")]);
        dictionary.tw_variants = dict(&[("喫", "吃")]);
        let opencc = opencc_with_dictionary(dictionary, &["純喫茶"]);

        assert_eq!(opencc.t2tw("純喫茶"), "純喫茶");
        assert_eq!(opencc.t2tw("喫"), "吃");
    }

    #[test]
    fn s2twp_combines_taiwan_dictionaries_in_second_round() {
        let mut dictionary = Dictionary::default();
        dictionary.tw_phrases = dict(&[("甲", "乙")]);
        dictionary.tw_variants_phrases = dict(&[("甲", "丁")]);
        dictionary.tw_variants = dict(&[("乙", "丙")]);
        let opencc = opencc_with_dictionary(dictionary, &["甲", "乙"]);

        // Taiwan phrases have first priority within round 2, and their output
        // is not fed through a separate Taiwan-variant round afterward.
        assert_eq!(opencc.s2twp("甲", false), "乙");
    }

    #[test]
    fn t2twp_uses_one_pass_with_dictionary_precedence() {
        let mut dictionary = Dictionary::default();
        dictionary.tw_phrases = dict(&[("甲", "乙")]);
        dictionary.tw_variants_phrases = dict(&[("甲", "丁")]);
        dictionary.tw_variants = dict(&[("乙", "丙")]);
        let opencc = opencc_with_dictionary(dictionary, &["甲", "乙"]);

        assert_eq!(opencc.t2twp("甲"), "乙");
    }

    #[test]
    fn tw2tp_uses_one_pass_with_dictionary_precedence() {
        let mut dictionary = Dictionary::default();
        dictionary.tw_variants_rev = dict(&[("甲", "乙")]);
        dictionary.tw_variants_rev_phrases = dict(&[("甲", "丁")]);
        dictionary.tw_phrases_rev = dict(&[("甲", "戊"), ("乙", "丙")]);
        let opencc = opencc_with_dictionary(dictionary, &["甲", "乙"]);

        assert_eq!(opencc.tw2tp("甲"), "乙");
    }

    #[test]
    fn hk_forward_variants_apply_phrases_before_characters() {
        let mut dictionary = Dictionary::default();
        dictionary.hk_variants_phrases = dict(&[("無線新聞", "無綫新聞")]);
        dictionary.hk_variants = dict(&[("線", "綫")]);
        let opencc = opencc_with_dictionary(dictionary, &["無線新聞"]);

        assert_eq!(opencc.t2hk("無線新聞"), "無綫新聞");
        assert_eq!(opencc.t2hk("線"), "綫");
    }

    #[test]
    fn t2hkp_uses_one_pass_with_dictionary_precedence() {
        let mut dictionary = Dictionary::default();
        dictionary.hk_phrases = dict(&[("甲", "乙")]);
        dictionary.hk_variants_phrases = dict(&[("甲", "丁")]);
        dictionary.hk_variants = dict(&[("乙", "丙")]);
        let opencc = opencc_with_dictionary(dictionary, &["甲", "乙"]);

        assert_eq!(opencc.t2hkp("甲"), "乙");
    }

    #[test]
    fn hk2tp_uses_one_pass_with_dictionary_precedence() {
        let mut dictionary = Dictionary::default();
        dictionary.hk_variants_rev_phrases = dict(&[("甲", "乙")]);
        dictionary.hk_variants_rev = dict(&[("甲", "丁")]);
        dictionary.hk_phrases_rev = dict(&[("甲", "戊"), ("乙", "丙")]);
        let opencc = opencc_with_dictionary(dictionary, &["甲", "乙"]);

        assert_eq!(opencc.hk2tp("甲"), "乙");
    }

    #[test]
    fn older_dictionary_json_defaults_new_forward_variant_phrase_slots() {
        let json = r#"{
            "schema_version": 2,
            "st_characters": {},
            "st_phrases": {},
            "ts_characters": {},
            "ts_phrases": {},
            "tw_phrases": {},
            "tw_phrases_rev": {},
            "tw_variants": {},
            "tw_variants_rev": {},
            "tw_variants_rev_phrases": {},
            "hk_variants": {},
            "hk_variants_rev": {},
            "hk_variants_rev_phrases": {},
            "jps_characters": {},
            "jps_phrases": {},
            "jp_variants": {},
            "jp_variants_rev": {}
        }"#;

        let dictionary: Dictionary = serde_json::from_str(json).unwrap();

        assert_eq!(dictionary.schema_version, 2);
        assert!(dictionary.tw_variants_phrases.is_empty());
        assert!(dictionary.hk_variants_phrases.is_empty());
    }

    #[test]
    fn schema_3_japanese_chains_use_shinjitai_slots() {
        let mut dictionary = Dictionary::default();
        dictionary.schema_version = 3;
        dictionary.jps_characters_rev = dict(&[("舊", "旧")]);
        dictionary.jps_characters = dict(&[("旧", "舊")]);
        dictionary.legacy_jp_variants = dict(&[("舊", "LEGACY")]);
        dictionary.legacy_jp_variants_rev = dict(&[("旧", "LEGACY")]);
        let opencc = opencc_with_dictionary(dictionary, &["舊", "旧"]);

        assert_eq!(opencc.t2jp("舊"), "旧");
        assert_eq!(opencc.jp2t("旧"), "舊");
    }

    #[test]
    fn schema_2_japanese_chains_keep_legacy_fallbacks() {
        let mut dictionary = Dictionary::default();
        dictionary.schema_version = 2;
        dictionary.legacy_jp_variants = dict(&[("舊", "旧")]);
        dictionary.legacy_jp_variants_rev = dict(&[("旧", "舊")]);
        let opencc = opencc_with_dictionary(dictionary, &["舊", "旧"]);

        assert_eq!(opencc.t2jp("舊"), "旧");
        assert_eq!(opencc.jp2t("旧"), "舊");
    }

    #[test]
    fn schema_3_serialization_uses_only_new_jp_hk_slots() {
        let mut dictionary = Dictionary::default();
        dictionary.schema_version = 3;
        let json = serde_json::to_string(&dictionary).unwrap();

        assert!(json.contains("\"hk_phrases\""));
        assert!(json.contains("\"hk_phrases_rev\""));
        assert!(json.contains("\"jps_characters_rev\""));
        assert!(!json.contains("\"jp_variants\""));
        assert!(!json.contains("\"jp_variants_rev\""));
    }

    #[test]
    fn bundled_dictionary_new_loads() {
        let dictionary = Dictionary::new();
        let json = serde_json::to_string(&dictionary).unwrap();

        assert_eq!(dictionary.schema_version, 3);
        assert!(!dictionary.st_characters.is_empty());
        assert!(!dictionary.hk_phrases.is_empty());
        assert!(!dictionary.hk_phrases_rev.is_empty());
        assert!(!dictionary.jps_characters_rev.is_empty());
        assert!(!json.contains("\"jp_variants\""));
        assert!(!json.contains("\"jp_variants_rev\""));
    }

    #[test]
    fn custom_dictionary_zstd_composes_with_jieba_user_dictionary() {
        let dictionary_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/dictionary_lib/dicts/dictionary.json.zst"
        );
        let user_dictionary_path = std::env::temp_dir().join(format!(
            "opencc-jieba-rs-user-dict-{}.txt",
            std::process::id()
        ));
        fs::write(&user_dictionary_path, "火星詞 100000 n\n").unwrap();

        let mut custom_first = OpenCC::try_new_with_dictionary_zstd(dictionary_path).unwrap();
        custom_first.load_user_dict(&user_dictionary_path).unwrap();
        assert_eq!(custom_first.s2t("汉字", false), "漢字");
        assert_eq!(custom_first.jieba_cut("火星詞", false), vec!["火星詞"]);

        let mut jieba_first = OpenCC::try_new_with_user_dict_path(&user_dictionary_path).unwrap();
        jieba_first.load_dictionary_zstd(dictionary_path).unwrap();
        assert_eq!(jieba_first.s2t("汉字", false), "漢字");
        assert_eq!(jieba_first.jieba_cut("火星詞", false), vec!["火星詞"]);

        fs::remove_file(user_dictionary_path).unwrap();
    }

    #[test]
    fn failed_custom_dictionary_load_preserves_current_dictionary() {
        let invalid_path = std::env::temp_dir().join(format!(
            "opencc-jieba-rs-invalid-dict-{}.zst",
            std::process::id()
        ));
        fs::write(&invalid_path, b"not a zstd dictionary").unwrap();

        let mut opencc = OpenCC::new();
        let before = opencc.s2t("汉字", false);
        let result = opencc.load_dictionary_zstd(&invalid_path);

        assert!(matches!(result, Err(OpenccError::DictionaryDecode(_))));
        assert_eq!(opencc.s2t("汉字", false), before);

        fs::remove_file(invalid_path).unwrap();
    }
}

/// Returns the maximum valid UTF-8 byte length for a string slice, ensuring no partial characters.
///
/// This function is useful when you need to truncate a string to a maximum byte count
/// without splitting multibyte UTF-8 characters.
///
/// # Arguments
///
/// * `sv` - The input string slice.
/// * `max_byte_count` - The maximum allowed byte count.
///
/// # Returns
///
/// The largest byte count less than or equal to `max_byte_count` that does not split a UTF-8 character.
///
/// # Examples
///
/// ```
/// use opencc_jieba_rs::find_max_utf8_length;
/// let s = "你好abc";
/// let max_len = find_max_utf8_length(s, 7);
/// assert_eq!(&s[..max_len], "你好a");
/// ```
pub fn find_max_utf8_length(sv: &str, max_byte_count: usize) -> usize {
    // 1. No longer than max byte count
    if sv.len() <= max_byte_count {
        return sv.len();
    }
    // 2. Longer than byte count
    let mut byte_count = max_byte_count;
    while byte_count > 0 && (sv.as_bytes()[byte_count] & 0b11000000) == 0b10000000 {
        byte_count -= 1;
    }
    byte_count
}

/// Removes line-break characters only when present, borrowing otherwise.
///
/// Keyword extraction APIs normalize CR/LF before passing text into
/// Jieba-based ranking. Returning [`Cow`] avoids allocating for the common
/// case where the input is already single-line.
#[inline]
fn strip_newlines_cow(input: &str) -> Cow<'_, str> {
    if input.as_bytes().iter().any(|&b| b == b'\n' || b == b'\r') {
        Cow::Owned(input.replace(['\n', '\r'], ""))
    } else {
        Cow::Borrowed(input)
    }
}

/// Validates and normalizes a Jieba user dictionary according to the
/// `opencc-jieba-rs` format contract.
///
/// This function enforces the crate's line format before passing data to
/// the underlying Jieba loader.
///
/// # Format
///
/// Each non-empty line must follow:
///
/// ```text
/// word freq [tag]
/// ```
///
/// - `word`: the token to insert into the dictionary
/// - `freq`: a required integer frequency
/// - `tag`: an optional part-of-speech tag
///
/// # Behavior
///
/// - Empty lines are ignored
/// - Leading/trailing whitespace is trimmed
/// - Lines are normalized into a newline-separated `String`
/// - The returned string is safe to pass into `jieba-rs`
///
/// # Errors
///
/// Returns [`OpenccError::UserDictParse`] if:
///
/// - a line does not contain 2 or 3 whitespace-separated fields
/// - the `freq` field is not a valid integer
///
/// Returns [`OpenccError::UserDictIo`] if:
///
/// - reading from the input fails
///
/// # Notes
///
/// This function intentionally requires an explicit integer frequency even
/// though `jieba-rs` may accept incomplete entries. This ensures predictable
/// behavior and avoids silent parsing issues.
fn validate_user_dict_format<R: BufRead>(reader: R) -> Result<String, OpenccError> {
    let mut validated = String::new();

    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(OpenccError::UserDictIo)?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        if parts.len() != 2 && parts.len() != 3 {
            return Err(OpenccError::UserDictParse(format!(
                "line {} invalid format: expected `word freq [tag]`",
                idx + 1
            )));
        }

        if parts[1].parse::<usize>().is_err() {
            return Err(OpenccError::UserDictParse(format!(
                "line {} invalid frequency `{}`",
                idx + 1,
                parts[1]
            )));
        }

        validated.push_str(trimmed);
        validated.push('\n');
    }

    Ok(validated)
}
