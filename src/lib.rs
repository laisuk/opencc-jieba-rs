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
//! The high-level text conversion methods take `&self` and `&str` input and
//! return an owned `String`.
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
//! ## Core Simplified ↔ Traditional
//!
//! [`OpenCC::s2t`] and [`OpenCC::t2s`] use phrase and character dictionaries
//! with internal Jieba segmentation. For configuration-driven conversion, use
//! [`OpenCC::convert`] or [`OpenCC::convert_with_config`]. Character-level
//! conversion helpers are internal implementation details.
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
//! Some Simplified/Traditional conversion methods accept a
//! `punctuation: bool` argument. When enabled, supported methods apply the
//! punctuation mapping after text conversion. Direct regional and variant
//! methods without that argument preserve punctuation.
//!
//! [`OpenCC::convert`] and [`OpenCC::convert_with_config`] accept the same
//! option generically, but ignore it for configurations whose direct method has
//! no punctuation conversion stage.
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

mod dictionary_lib;
mod keyword;
mod opencc;
mod opencc_config;

#[cfg(feature = "dictionary-build")]
pub mod dictionary_build;

pub use dictionary_lib::{CustomDictFileSpec, CustomDictMode, CustomDictSpec, DictSlot};
pub use jieba_rs::Keyword;
pub use keyword::{KeywordMethod, POS_KEYWORDS};
pub use opencc::{find_max_utf8_length, is_delimiter, OpenCC, OpenccError, UserDictEntry};
pub use opencc_config::OpenccConfig;

// Kept at the crate root for the internal keyword module's existing call path.
pub(crate) use opencc::strip_newlines_cow;
