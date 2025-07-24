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
use jieba_rs::{Jieba, Keyword, TfIdf};
use jieba_rs::{KeywordExtract, TextRank};
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::io::{Cursor, Read};
use std::ops::Range;
use std::sync::Arc;
use zstd::stream::read::Decoder;

use crate::dictionary_lib::Dictionary;
pub mod dictionary_lib;

const DICT_HANS_HANT_ZSTD: &[u8] = include_bytes!("dictionary_lib/dicts/dict_hans_hant.txt.zst");
static DELIMITER_SET: Lazy<HashSet<char>> = Lazy::new(|| {
    " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；："
        .chars()
        .collect()
});
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]").unwrap());
// Pre-compiled regexes using lazy static initialization
static S2T_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[“”‘’]"#).unwrap());
static T2S_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"[「」『』]").unwrap());
// Pre-built mapping tables
static S2T_MAP: Lazy<HashMap<char, char>> = Lazy::new(|| {
    [('“', '「'), ('”', '」'), ('‘', '『'), ('’', '』')]
        .into_iter()
        .collect()
});
static T2S_MAP: Lazy<HashMap<char, char>> = Lazy::new(|| {
    [('「', '“'), ('」', '”'), ('『', '‘'), ('』', '’')]
        .into_iter()
        .collect()
});
// Minimum input length (in chars) to trigger parallel processing
const PARALLEL_THRESHOLD: usize = 1000;

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
        let dict_hans_hant_bytes = decompress_jieba_dict();
        let mut dict_hans_hant = BufReader::new(Cursor::new(dict_hans_hant_bytes));
        let jieba = Arc::new(Jieba::with_dict(&mut dict_hans_hant).unwrap());
        let dictionary = Dictionary::new();

        OpenCC { jieba, dictionary }
    }

    // Performs dictionary-based phrase-level conversion with fallback character-level mapping.
    //
    // This is the core logic for segmenting and converting input text using Jieba and multiple
    // dictionaries. It supports both phrase-level and character-level matching across segmented
    // chunks, and can operate in parallel if the input is large.
    //
    // ## Workflow:
    // 1. Split input into ranges based on delimiters (e.g., punctuation).
    // 2. For each range, perform Jieba segmentation (`cut`) with or without HMM.
    // 3. For each segmented phrase:
    //    - If it's a known delimiter, return as-is.
    //    - Else, lookup phrase in each dictionary (short-circuiting on first match).
    //    - If not found in any dictionary, fallback to per-character conversion.
    //
    // ## Parallelism:
    // - Enabled if input length ≥ `PARALLEL_THRESHOLD`.
    // - Each range is processed in parallel and results are flattened into a single `String`.
    //
    // # Arguments
    // - `input`: The input text to convert.
    // - `dictionaries`: Slice of reference-to-dictionaries (ordered by priority).
    // - `hmm`: Whether to enable HMM in Jieba segmentation.
    //
    // # Returns
    // - A fully converted `String`, combining segment-level and character-level replacements.
    //
    // Example use cases: s2t, t2s, s2tw conversions with phrase-first dictionary application.
    fn phrases_cut_convert<'a>(
        &'a self,
        input: &'a str,
        dictionaries: &'a [&HashMap<String, String>],
        hmm: bool,
    ) -> String {
        let ranges = self.split_string_ranges(input, true);
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            self.jieba
                .cut(chunk, hmm)
                .into_iter()
                .map(|phrase| {
                    let mut chars = phrase.chars();
                    if let (Some(c), None) = (chars.next(), chars.next()) {
                        if DELIMITER_SET.contains(&c) {
                            return phrase.to_string();
                        }
                    }

                    for dict in dictionaries {
                        if let Some(translated) = dict.get(phrase) {
                            return translated.clone();
                        }
                    }

                    Self::convert_by_char(phrase, dictionaries, false)
                })
                .collect::<Vec<String>>()
        };

        if use_parallel {
            String::from_par_iter(ranges.into_par_iter().flat_map_iter(process_range))
        } else {
            String::from_iter(ranges.into_iter().flat_map(process_range))
        }
    }

    // Fallback character-by-character dictionary conversion.
    //
    // Used when a phrase is not matched in any dictionary during segmentation.
    // Each character is individually looked up in the same dictionary list.
    //
    // Supports optional parallelization for long strings.
    //
    // # Arguments
    // - `phrase`: The phrase to convert (typically short).
    // - `dictionaries`: Ordered list of dictionaries to apply.
    // - `use_parallel`: Whether to enable parallel processing.
    //
    // # Returns
    // - A `String` where each char is replaced using the first matching dictionary.
    fn convert_by_char(
        phrase: &str,
        dictionaries: &[&HashMap<String, String>],
        use_parallel: bool,
    ) -> String {
        let process = |ch: char| {
            let mut buf = [0u8; 4];
            let ch_str = ch.encode_utf8(&mut buf);

            for dict in dictionaries {
                if let Some(t) = dict.get(ch_str) {
                    return t.clone(); // May be multi-char
                }
            }

            ch_str.to_owned()
        };

        if use_parallel {
            phrase.par_chars().map(&process).collect::<String>()
        } else {
            let mut result = String::with_capacity(phrase.len() * 4); // Estimate
            for ch in phrase.chars() {
                result.push_str(&process(ch));
            }
            result
        }
    }

    /// Splits text into non-overlapping ranges between delimiter characters.
    ///
    /// Each `Range<usize>` corresponds to a segment bounded by or ending at a delimiter,
    /// depending on the `inclusive` parameter.
    ///
    /// - If `inclusive` is true, the delimiter is included at the end of each range.
    /// - If `inclusive` is false, the delimiter starts the next range (excluded from current).
    ///
    /// Example:
    ///   Input:  "你好，世界！Rust不错。"
    ///   Output (inclusive=true):  vec![0..9, 9..18, 18..27, ...]
    ///   Output (inclusive=false): vec![0..6, 6..15, 15..24, ...]
    fn split_string_ranges(&self, text: &str, inclusive: bool) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();
        let mut start = 0;

        for (i, &(_, ch)) in char_indices.iter().enumerate() {
            if DELIMITER_SET.contains(&ch) {
                let ch_start = char_indices[i].0;
                let ch_end = if i + 1 < char_indices.len() {
                    char_indices[i + 1].0
                } else {
                    text.len()
                };

                if inclusive {
                    ranges.push(start..ch_end);
                } else {
                    if start < ch_start {
                        ranges.push(start..ch_start);
                    }
                    ranges.push(ch_start..ch_end);
                }

                start = ch_end; // Moved out safely
            }
        }

        if start < text.len() {
            ranges.push(start..text.len());
        }

        ranges
    }

    // Performs Jieba-based phrase segmentation over each non-delimiter chunk.
    // Used internally for consistent pre-segmentation before conversion or keyword extraction.
    fn phrases_cut_impl(&self, input: &str, hmm: bool, use_parallel: bool) -> Vec<String> {
        let ranges = self.split_string_ranges(input, true);

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            self.jieba
                .cut(chunk, hmm)
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<String>>()
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

    /// Segments input text using Jieba tokenizer.
    ///
    /// # Arguments
    ///
    /// * `text` - The input text to be segmented.
    /// * `hmm` - Whether to enable HMM for unknown word recognition.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` containing segmented words.
    ///
    /// # Example
    /// ```
    /// let opencc = opencc_jieba_rs::OpenCC::new();
    /// let tokens = opencc.jieba_cut("南京市长江大桥", true);
    /// assert!(tokens.contains(&"南京市".to_string()));  // "南京市/长江大桥"
    /// ```
    pub fn jieba_cut(&self, input: &str, hmm: bool) -> Vec<String> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_cut_impl(input, hmm, use_parallel)
    }

    /// Segments input text using Jieba and joins the result into a single string.
    ///
    /// Similar to [`jieba_cut`] but returns a space-separated string instead of a vector.
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
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        );
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
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        );
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    /// Converts Simplified Chinese to Traditional Chinese (Taiwan) with punctuation.
    ///
    /// Performs a full conversion of text and punctuation marks from Simplified
    /// to Traditional Chinese, including quote styles (`“”` → `「」`).
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
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(
                &self.phrases_cut_convert(input, &dict_refs, true),
                &dict_refs_round_2,
                true,
            ),
            &dict_refs_round_3,
            true,
        );
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
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let dict_refs_round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(
                &self.phrases_cut_convert(input, &dict_refs, true),
                &dict_refs_round_2,
                true,
            ),
            &dict_refs_round_3,
            true,
        );
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
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        );
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let result = self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        );
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    pub fn t2tw(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.tw_variants];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    pub fn t2twp(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.tw_phrases];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        )
    }

    pub fn tw2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    pub fn tw2tp(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        self.phrases_cut_convert(
            &self.phrases_cut_convert(input, &dict_refs, true),
            &dict_refs_round_2,
            true,
        )
    }

    pub fn t2hk(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.hk_variants];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    pub fn hk2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    pub fn t2jp(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.jp_variants];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    pub fn jp2t(&self, input: &str) -> String {
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        self.phrases_cut_convert(input, &dict_refs, true)
    }

    // Fast character-level Simplified → Traditional Chinese conversion.
    //
    // Uses only the `st_characters` dictionary (no segmentation).
    // Optimized for scenarios where fine-grained phrase matching is unnecessary.
    //
    // Parallelized via `convert_by_char()` for speed on large inputs.
    //
    // Example use case: punctuation or pure character-level normalization.
    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let output = Self::convert_by_char(input, &dict_refs, true);
        output
    }

    // Fast character-level Traditional → Simplified Chinese conversion.
    //
    // Uses only the `ts_characters` dictionary (no segmentation).
    // Ideal for bulk character-wise normalization tasks, skipping phrase context.
    //
    // Parallel execution is enabled by default.
    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let output = Self::convert_by_char(input, &dict_refs, true);
        output
    }

    /// Converts Chinese text between different variants using a specified conversion configuration.
    ///
    /// This is the core function for text conversion. It supports conversion between Simplified, Traditional,
    /// Taiwanese, Hong Kong, and Japanese Chinese variants, as well as punctuation conversion.
    ///
    /// # Arguments
    ///
    /// * `input` - The input string to be converted.
    /// * `config` - The conversion configuration. Supported values (case-insensitive) include:
    ///     - `"s2t"`: Simplified to Traditional
    ///     - `"s2tw"`: Simplified to Taiwanese
    ///     - `"s2twp"`: Simplified to Taiwanese (with phrases)
    ///     - `"s2hk"`: Simplified to Hong Kong
    ///     - `"t2s"`: Traditional to Simplified
    ///     - `"t2tw"`: Traditional to Taiwanese
    ///     - `"t2twp"`: Traditional to Taiwanese (with phrases)
    ///     - `"t2hk"`: Traditional to Hong Kong
    ///     - `"tw2s"`: Taiwanese to Simplified
    ///     - `"tw2sp"`: Taiwanese to Simplified (with phrases)
    ///     - `"tw2t"`: Taiwanese to Traditional
    ///     - `"tw2tp"`: Taiwanese to Traditional (with phrases)
    ///     - `"hk2s"`: Hong Kong to Simplified
    ///     - `"hk2t"`: Hong Kong to Traditional
    ///     - `"jp2t"`: Japanese to Traditional
    ///     - `"t2jp"`: Traditional to Japanese
    /// * `punctuation` - Whether to convert punctuation marks according to the target variant.
    ///
    /// # Returns
    ///
    /// A `String` containing the converted text. If the `config` is invalid, returns an error message.
    ///
    /// # Examples
    ///
    /// ```
    /// use opencc_jieba_rs::OpenCC;
    /// let opencc = OpenCC::new();
    /// let traditional = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "s2t", true);
    /// let taiwanese = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "s2tw", true);
    /// let invalid = opencc.convert("“春眠不觉晓，处处闻啼鸟。”", "unknown", true);
    /// assert_eq!(invalid, "Invalid config: unknown");
    /// ```
    pub fn convert(&self, input: &str, config: &str, punctuation: bool) -> String {
        match config.to_lowercase().as_str() {
            "s2t" => self.s2t(input, punctuation),
            "s2tw" => self.s2tw(input, punctuation),
            "s2twp" => self.s2twp(input, punctuation),
            "s2hk" => self.s2hk(input, punctuation),
            "t2s" => self.t2s(input, punctuation),
            "t2tw" => self.t2tw(input),
            "t2twp" => self.t2twp(input),
            "t2hk" => self.t2hk(input),
            "tw2s" => self.tw2s(input, punctuation),
            "tw2sp" => self.tw2sp(input, punctuation),
            "tw2t" => self.tw2t(input),
            "tw2tp" => self.tw2tp(input),
            "hk2s" => self.hk2s(input, punctuation),
            "hk2t" => self.hk2t(input),
            "jp2t" => self.jp2t(input),
            "t2jp" => self.t2jp(input),
            _ => format!("Invalid config: {}", config),
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
        let _strip_text = STRIP_REGEX.replace_all(input, "");
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
        let (regex, mapping) = if config.starts_with('s') {
            (&*S2T_REGEX, &*S2T_MAP)
        } else {
            (&*T2S_REGEX, &*T2S_MAP)
        };

        regex
            .replace_all(text, |caps: &regex::Captures| {
                let ch = caps.get(0).unwrap().as_str().chars().next().unwrap();
                mapping.get(&ch).copied().unwrap_or(ch).to_string()
            })
            .into_owned()
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
    pub fn keyword_extract_textrank(&self, input: &str, tok_k: usize) -> Vec<String> {
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TextRank::default();
        let top_k = keyword_extractor.extract_keywords(
            &self.jieba,
            &cleaned_input,
            tok_k,
            // vec![String::from("ns"), String::from("n"), String::from("vn"), String::from("v")],
            vec![],
        );
        // Extract only the keyword strings from the Keyword struct
        top_k.into_iter().map(|k| k.keyword).collect()
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
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TextRank::default();
        let top_k = keyword_extractor.extract_keywords(
            &self.jieba,
            &cleaned_input,
            top_k,
            // vec![String::from("ns"), String::from("n"), String::from("vn"), String::from("v")],
            vec![],
        );
        top_k
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
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TfIdf::default();
        let top_k = keyword_extractor.extract_keywords(&self.jieba, &cleaned_input, top_k, vec![]);
        // Extract only the keyword strings from the Keyword struct
        top_k.into_iter().map(|k| k.keyword).collect()
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
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TfIdf::default();
        let top_k = keyword_extractor.extract_keywords(&self.jieba, &cleaned_input, top_k, vec![]);

        top_k
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

/// Decompresses the embedded Jieba dictionary using Zstandard compression.
///
/// This function loads the compressed dictionary from the binary (`DICT_HANS_HANT_ZSTD`),
/// decompresses it using the `zstd` crate, and returns the raw bytes of the dictionary
/// data without performing UTF-8 validation or conversion.
///
/// This is used internally for initializing Jieba with a dictionary reader.
///
/// # Panics
///
/// Panics if decompression fails or the dictionary cannot be read into memory.
///
/// # Returns
///
/// A `Vec<u8>` containing the decompressed dictionary data as raw bytes.
fn decompress_jieba_dict() -> Vec<u8> {
    let cursor = Cursor::new(DICT_HANS_HANT_ZSTD);
    let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .expect("Failed to decompress dictionary");
    decompressed
}
