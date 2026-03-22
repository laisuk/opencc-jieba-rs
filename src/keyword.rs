//! Keyword extraction utilities based on Jieba.
//!
//! This module provides internal implementations and supporting types for
//! keyword extraction using the following algorithms:
//!
//! - **TextRank** — a graph-based ranking algorithm based on word co-occurrence
//! - **TF-IDF** — a statistical method based on term frequency and inverse document frequency
//!
//! These implementations are used by the public [`OpenCC`](crate::OpenCC) API
//! to provide keyword extraction with optional part-of-speech (POS) filtering.
//!
//! # Features
//!
//! - Unified internal implementation for multiple keyword extraction methods
//! - Optional POS filtering to improve keyword quality
//! - Support for both plain (`Vec<String>`) and weighted (`Vec<Keyword>`) outputs
//!
//! # POS Filtering
//!
//! Keyword extraction functions in this crate support optional filtering by
//! part-of-speech (POS) tags. This helps remove low-information tokens such as
//! adverbs and particles, improving the quality of extracted keywords.
//!
//! A recommended preset is provided:
//!
//! - [`POS_KEYWORDS`] — common nouns and verbs for general-purpose keyword extraction
//!
//! # Usage
//!
//! Most users should use the high-level methods provided by [`OpenCC`](crate::OpenCC):
//!
//! ```
//! use opencc_jieba_rs::{OpenCC, POS_KEYWORDS};
//!
//! let opencc = OpenCC::new();
//!
//! let keywords = opencc.keyword_extract_textrank_pos(
//!     "自然语言处理和机器学习",
//!     5,
//!     Some(POS_KEYWORDS),
//! );
//!
//! println!("{:?}", keywords);
//! ```
//!
//! # Design Notes
//!
//! - This module centralizes keyword extraction logic to avoid duplication.
//! - Internal functions are marked `pub(crate)` and are not part of the public API.
//! - POS filters are accepted as `&[&str]` for zero-allocation input at the API level,
//!   and converted internally to match the `jieba-rs` interface.
//!
//! # See also
//!
//! - [`KeywordMethod`] — selects the extraction algorithm
//! - [`POS_KEYWORDS`] — recommended POS filter preset
//! - [`OpenCC::keyword_extract_textrank`](crate::OpenCC::keyword_extract_textrank)
//! - [`OpenCC::keyword_extract_tfidf`](crate::OpenCC::keyword_extract_tfidf)

use jieba_rs::{Keyword, KeywordExtract, TextRank, TfIdf};

/// Keyword extraction algorithm.
///
/// Specifies which algorithm to use when extracting keywords from text.
///
/// # Variants
///
/// - [`TextRank`](KeywordMethod::TextRank)
///   A graph-based ranking algorithm that evaluates word importance based on
///   co-occurrence relationships within the text.
///
/// - [`TfIdf`](KeywordMethod::TfIdf)
///   A statistical method that ranks words based on their frequency in the input
///   text and their inverse frequency across a corpus.
///
/// # Usage
///
/// This enum is used internally by keyword extraction functions to select
/// the desired algorithm. Most users will interact with higher-level methods
/// such as:
///
/// ```
/// use opencc_jieba_rs::OpenCC;
///
/// let opencc = OpenCC::new();
///
/// let textrank = opencc.keyword_extract_textrank("自然语言处理和机器学习", 5);
/// let tfidf = opencc.keyword_extract_tfidf("自然语言处理和机器学习", 5);
///
/// println!("TextRank: {:?}", textrank);
/// println!("TF-IDF: {:?}", tfidf);
/// ```
///
/// # Notes
///
/// - `TextRank` is generally better for capturing semantic relationships
///   in natural language text.
/// - `TfIdf` is more suitable for domain-specific or technical content
///   where term frequency is a strong signal.
///
/// # See also
///
/// - [`OpenCC::keyword_extract_textrank`](crate::OpenCC::keyword_extract_textrank)
/// - [`OpenCC::keyword_extract_tfidf`](crate::OpenCC::keyword_extract_tfidf)
/// - [`POS_KEYWORDS`](POS_KEYWORDS)
///
/// # Since
/// v0.7.4
#[derive(Debug, Clone, Copy)]
pub enum KeywordMethod {
    /// Graph-based keyword ranking using word co-occurrence.
    TextRank,

    /// Statistical keyword ranking based on TF-IDF scores.
    TfIdf,
}

/// Recommended part-of-speech (POS) tags for keyword extraction.
///
/// This preset includes common **content-bearing words** such as nouns and verbs,
/// which typically produce more meaningful keyword extraction results.
///
/// # Included POS tags
///
/// - `"n"`  — common nouns
/// - `"nr"` — person names
/// - `"ns"` — place names
/// - `"nt"` — organization names
/// - `"nz"` — other proper nouns
/// - `"v"`  — verbs
/// - `"vn"` — verb-noun compounds
///
/// # Usage
///
/// ```
/// use opencc_jieba_rs::{OpenCC, POS_KEYWORDS};
///
/// let opencc = OpenCC::new();
///
/// let keywords = opencc.keyword_extract_textrank_pos(
///     "自然语言处理和机器学习",
///     5,
///     Some(POS_KEYWORDS),
/// );
///
/// println!("{:?}", keywords);
/// ```
///
/// # Notes
///
/// - This preset is designed to **filter out low-information tokens** such as
///   adverbs, particles, and function words.
/// - It generally improves keyword quality for:
///   - news articles
///   - general prose
///   - technical documents
///
/// - You may customize POS filters depending on your use case:
///   - Add `"a"` for adjectives if descriptive terms are important
///   - Remove `"v"` if only entities (nouns) are desired
///
/// # See also
///
/// - [`keyword_extract_textrank_pos`](crate::OpenCC::keyword_extract_textrank_pos)
/// - [`keyword_extract_tfidf_pos`](crate::OpenCC::keyword_extract_tfidf_pos)
///
/// # Since
/// v0.7.4
pub const POS_KEYWORDS: &[&str] = &["n", "nr", "ns", "nt", "nz", "v", "vn"];

/// Internal keyword extraction implementation shared by all public APIs.
///
/// This function performs keyword extraction using the specified [`KeywordMethod`],
/// optionally applying part-of-speech (POS) filtering. It is used internally by
/// higher-level `OpenCC` methods to avoid code duplication.
///
/// # Arguments
///
/// * `jieba` - Reference to the underlying Jieba tokenizer.
/// * `input` - Input text to extract keywords from.
/// * `top_k` - Maximum number of keywords to return.
/// * `method` - Keyword extraction algorithm to use (e.g. `TextRank`, `TfIdf`).
/// * `allowed_pos` - Optional slice of POS tags used to filter candidate words.
///   - `None` means no filtering.
///   - `Some(&["n", "nr", ...])` restricts extraction to specific POS categories.
///
/// # Returns
///
/// A `Vec<String>` containing extracted keywords sorted by importance.
///
/// # Implementation notes
///
/// - Newline characters are removed from the input before processing.
/// - POS filters are converted to `Vec<String>` to match the underlying
///   `jieba-rs` API requirements.
/// - This function centralizes shared logic for both TextRank and TF-IDF
///   keyword extraction, ensuring consistent behavior across all variants.
///
/// # Usage
///
/// This is an internal helper and should not be called directly.
/// Use the public `OpenCC` methods instead:
///
/// - [`OpenCC::keyword_extract_textrank`](crate::OpenCC::keyword_extract_textrank)
/// - [`OpenCC::keyword_extract_textrank_pos`](crate::OpenCC::keyword_extract_textrank_pos)
/// - [`OpenCC::keyword_extract_tfidf`](crate::OpenCC::keyword_extract_tfidf)
/// - [`OpenCC::keyword_extract_tfidf_pos`](crate::OpenCC::keyword_extract_tfidf_pos)
///
/// # Since
/// v0.7.4
pub(crate) fn keyword_extract_internal(
    jieba: &jieba_rs::Jieba,
    input: &str,
    top_k: usize,
    method: KeywordMethod,
    allowed_pos: Option<&[&str]>,
) -> Vec<String> {
    let cleaned_input = crate::strip_newlines_cow(input);

    let pos_vec: Vec<String> = allowed_pos
        .map(|v| v.iter().map(|&s| s.to_string()).collect())
        .unwrap_or_default();

    let keywords = match method {
        KeywordMethod::TextRank => {
            let extractor = TextRank::default();
            extractor.extract_keywords(jieba, &cleaned_input, top_k, pos_vec)
        }
        KeywordMethod::TfIdf => {
            let extractor = TfIdf::default();
            extractor.extract_keywords(jieba, &cleaned_input, top_k, pos_vec)
        }
    };

    keywords.into_iter().map(|k| k.keyword).collect()
}

/// Internal weighted keyword extraction using the TextRank algorithm.
///
/// This function performs keyword extraction using TextRank and returns
/// structured [`Keyword`] objects containing both the keyword string and
/// its associated weight. It optionally applies part-of-speech (POS) filtering
/// to restrict candidate words.
///
/// # Arguments
///
/// * `jieba` - Reference to the underlying Jieba tokenizer.
/// * `input` - Input text to analyze.
/// * `top_k` - Maximum number of keywords to return.
/// * `allowed_pos` - Optional slice of POS tags used to filter candidate words.
///   - `None` means no filtering.
///   - `Some(&["n", "nr", ...])` restricts extraction to specific POS categories.
///
/// # Returns
///
/// A `Vec<Keyword>` where each element contains:
/// - `keyword`: The extracted word.
/// - `weight`: The TextRank score representing its importance.
///
/// # Implementation notes
///
/// - Newline characters are removed from the input before processing.
/// - POS filters are converted to `Vec<String>` to match the underlying
///   `jieba-rs` API requirements.
/// - This function is shared by public APIs to avoid duplication and ensure
///   consistent behavior.
///
/// # Usage
///
/// This is an internal helper and should not be called directly.
/// Use the public `OpenCC` methods instead:
///
/// - [`OpenCC::keyword_weight_textrank`](crate::OpenCC::keyword_weight_textrank)
/// - [`OpenCC::keyword_weight_textrank_pos`](crate::OpenCC::keyword_weight_textrank_pos)
///
/// [`Keyword`]: https://docs.rs/jieba-rs/latest/jieba_rs/struct.Keyword.html
///
/// # Since
/// v0.7.4
pub(crate) fn keyword_weight_textrank_internal(
    jieba: &jieba_rs::Jieba,
    input: &str,
    top_k: usize,
    allowed_pos: Option<&[&str]>,
) -> Vec<Keyword> {
    let cleaned_input = crate::strip_newlines_cow(input);
    let keyword_extractor = TextRank::default();

    let pos_vec: Vec<String> = allowed_pos
        .map(|v| v.iter().map(|&s| s.to_string()).collect())
        .unwrap_or_default();

    keyword_extractor.extract_keywords(jieba, &cleaned_input, top_k, pos_vec)
}

/// Internal weighted keyword extraction using the TF-IDF algorithm.
///
/// This function performs keyword extraction using TF-IDF and returns
/// structured [`Keyword`] objects containing both the keyword string and
/// its associated weight. It optionally applies part-of-speech (POS) filtering
/// to restrict candidate words.
///
/// # Arguments
///
/// * `jieba` - Reference to the underlying Jieba tokenizer.
/// * `input` - Input text to analyze.
/// * `top_k` - Maximum number of keywords to return.
/// * `allowed_pos` - Optional slice of POS tags used to filter candidate words.
///   - `None` means no filtering.
///   - `Some(&["n", "nr", ...])` restricts extraction to specific POS categories.
///
/// # Returns
///
/// A `Vec<Keyword>` where each element contains:
/// - `keyword`: The extracted word.
/// - `weight`: The TF-IDF score representing its importance.
///
/// # Implementation notes
///
/// - Newline characters are removed from the input before processing.
/// - POS filters are converted to `Vec<String>` to match the underlying
///   `jieba-rs` API requirements.
/// - This function is shared by public APIs to avoid duplication and ensure
///   consistent behavior.
///
/// # Usage
///
/// This is an internal helper and should not be called directly.
/// Use the public `OpenCC` methods instead:
///
/// - [`OpenCC::keyword_weight_tfidf`](crate::OpenCC::keyword_weight_tfidf)
/// - [`OpenCC::keyword_weight_tfidf_pos`](crate::OpenCC::keyword_weight_tfidf_pos)
///
/// [`Keyword`]: https://docs.rs/jieba-rs/latest/jieba_rs/struct.Keyword.html
///
/// # Since
/// v0.7.4
pub(crate) fn keyword_weight_tfidf_internal(
    jieba: &jieba_rs::Jieba,
    input: &str,
    top_k: usize,
    allowed_pos: Option<&[&str]>,
) -> Vec<Keyword> {
    let cleaned_input = crate::strip_newlines_cow(input);
    let keyword_extractor = TfIdf::default();

    let pos_vec: Vec<String> = allowed_pos
        .map(|v| v.iter().map(|&s| s.to_string()).collect())
        .unwrap_or_default();

    keyword_extractor.extract_keywords(jieba, &cleaned_input, top_k, pos_vec)
}
