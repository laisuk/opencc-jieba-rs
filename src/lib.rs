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
use std::collections::HashMap;
use std::io::BufReader;
use std::io::{Cursor, Read};
use std::ops::Range;
use std::sync::Arc;
use zstd::stream::read::Decoder;

use crate::dictionary_lib::{DictMap, Dictionary};
pub mod dictionary_lib;

const DICT_HANS_HANT_ZSTD: &[u8] = include_bytes!("dictionary_lib/dicts/dict_hans_hant.txt.zst");

// static DELIMITER_SET: Lazy<HashSet<char>> = Lazy::new(|| {
//     " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；："
//         .chars()
//         .collect()
// });

/// Master delimiter string containing all punctuation and whitespace
/// considered as token boundaries by OpenCC-Jieba.
///
/// This includes:
/// - ASCII whitespace (`' '`, `\t`, `\n`, `\r`) and symbols (`! " # $ …`).
/// - Common CJK punctuation (、 。 「 」 『 』 《 》 【 】, etc.).
/// - Fullwidth variants of Latin punctuation (e.g., `，；：？！～`).
///
/// It is used at build/initialization time to precompute the delimiter bitmap.
///
/// You generally should not use this constant directly—prefer
/// [`is_delimiter()`](fn.is_delimiter.html) for fast lookup.
const DELIMITER_STR: &str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
/// 65,536 bits for the BMP (U+0000..=U+FFFF) → 1024 * u64
static DELIM_BMP: Lazy<[u64; 1024]> = Lazy::new(|| {
    let mut bm = [0u64; 1024];
    for ch in DELIMITER_STR.chars() {
        let u = ch as u32;
        if u <= 0x_FFFF {
            let idx = (u >> 6) as usize; // which u64
            let bit = u & 63; // which bit
            bm[idx] |= 1u64 << bit;
        }
    }
    bm
});

/// Tests whether a character is a delimiter.
///
/// This function performs a **constant-time bitmap lookup** for any BMP
/// character (`U+0000..=U+FFFF`), using the precomputed [`DELIM_BMP`] table.
/// For non-BMP code points, it currently always returns `false`
/// (extend this if you need astral punctuation).
///
/// # Arguments
///
/// * `c` – A [`char`] to test.
///
/// # Returns
///
/// * `true` if `c` is considered a delimiter (whitespace, punctuation, etc.).
/// * `false` otherwise.
///
/// # Performance
///
/// - Single array access + bit test (branch-free in the hot path).
/// - Much faster than [`HashSet<char>`](std::collections::HashSet) lookup.
/// - Table fits in L1 cache (~8 KB).
///
/// # Examples
///
/// ```
/// use opencc_jieba_rs::is_delimiter;
///
/// assert!(is_delimiter('。')); // ideographic full stop
/// assert!(is_delimiter(' '));  // ASCII space
/// assert!(!is_delimiter('你')); // normal character
/// ```
#[inline(always)]
pub fn is_delimiter(c: char) -> bool {
    let u = c as u32;
    if u <= 0x_FFFF {
        // SAFETY: array is fixed-size, index is 0..1023 for BMP
        let word = unsafe { *DELIM_BMP.get_unchecked((u >> 6) as usize) };
        ((word >> (u & 63)) & 1) != 0
    } else {
        // Most CJK punctuation is BMP; add astral checks if needed them later.
        false
    }
}

// Pre-compiled regexes using lazy static initialization
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_著]").unwrap());
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

        #[inline(always)]
        fn single_and_len(s: &str) -> (bool, Option<char>, u16) {
            let mut it = s.chars();
            match (it.next(), it.next()) {
                (None, _) => (true, None, 0),          // empty
                (Some(c), None) => (true, Some(c), 1), // exactly 1 char
                (Some(_), Some(_)) => (false, None, 2 + it.count() as u16),
            }
        }

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            let tokens = self.jieba.cut(chunk, hmm);

            // Heuristic: output ~= input in length
            let mut out = String::with_capacity(chunk.len());

            'tok: for phrase in tokens {
                if phrase.is_empty() {
                    continue 'tok;
                }

                // Get (is_single, optional_char, total_length)
                let (is_single, single_char_opt, phrase_len) = single_and_len(phrase);

                // Fast delimiter path
                if is_single {
                    if let Some(c) = single_char_opt {
                        if is_delimiter(c) {
                            out.push_str(phrase);
                            continue 'tok;
                        }
                    }
                    Self::convert_by_char(phrase, dictionaries, &mut out);
                    continue 'tok;
                }

                // Precedence lookup across dicts
                for dict in dictionaries {
                    if !dict.has_key_len(phrase_len) {
                        continue;
                    }
                    if let Some(t) = dict.get(phrase) {
                        out.push_str(t);
                        continue 'tok;
                    }
                }

                // Fallback: char-by-char conversion, in-place
                Self::convert_by_char(phrase, dictionaries, &mut out);
            }

            out
        };

        if use_parallel {
            ranges
                .into_par_iter()
                .map(process_range)
                .reduce(String::new, |mut a, b| {
                    a.reserve(b.len());
                    a.push_str(&b);
                    a
                })
        } else {
            let mut out = String::with_capacity(input.len() + (input.len() >> 6));
            for r in ranges {
                out.push_str(&process_range(r));
            }
            out
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
                if dict.min_len > 1 {
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
    /// This method is used internally to pre-segment input text into a vector of phrases
    /// before dictionary-based conversion or keyword extraction. It ensures consistent
    /// segmentation behavior across single-threaded and parallel execution modes.
    ///
    /// The segmentation respects delimiters (such as punctuation or whitespace) by first
    /// splitting the text into non-delimiter ranges using [`split_string_ranges()`]. Each
    /// range is then processed by Jieba’s `cut()` function, producing an ordered sequence
    /// of token strings.
    ///
    /// When `use_parallel` is enabled, segmentation is parallelized using Rayon’s
    /// [`IndexedParallelIterator`], which preserves the global order of tokens while
    /// distributing work across CPU cores. This improves throughput on large texts
    /// without affecting deterministic output ordering.
    ///
    /// # Arguments
    /// - `input` — The input text to segment.
    /// - `hmm` — Whether to enable Jieba’s Hidden Markov Model (HMM) for unknown word detection.
    /// - `use_parallel` — Whether to perform segmentation in parallel using Rayon.
    ///
    /// # Returns
    /// A vector of segmented phrase strings (`Vec<String>`), preserving the order
    /// of appearance in the input.
    ///
    /// # Notes
    /// - Each phrase corresponds to a Jieba token.
    /// - Global order of phrases is guaranteed even in parallel mode.
    /// - Delimiters are handled separately by [`split_string_ranges()`].
    fn phrases_cut_impl(&self, input: &str, hmm: bool, use_parallel: bool) -> Vec<String> {
        let ranges = self.split_string_ranges(input, true);

        let process_range = |range: Range<usize>| {
            let chunk = &input[range];
            self.jieba
                .cut(chunk, hmm) // Vec<&str>
                .into_iter()
                .map(str::to_owned)
        };

        if use_parallel {
            // Because `ranges.into_par_iter()` is an IndexedParallelIterator and each inner
            // iterator is ExactSizeIterator, Rayon preserves global order when collecting.
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
    // Example use case: punctuation or pure character-level normalization.
    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let mut output = String::with_capacity(input.len());
        Self::convert_by_char(input, &dict_refs, &mut output);
        output
    }

    // Fast character-level Traditional → Simplified Chinese conversion.
    //
    // Uses only the `ts_characters` dictionary (no segmentation).
    // Ideal for bulk character-wise normalization tasks, skipping phrase context.
    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let mut output = String::with_capacity(input.len());
        Self::convert_by_char(input, &dict_refs, &mut output);
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
        let check_length = find_max_utf8_length(input, 1000);
        let _strip_text = STRIP_REGEX.replace_all(&input[..check_length], "");
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
