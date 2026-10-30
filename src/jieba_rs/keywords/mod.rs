mod ordered_float;
use crate::jieba_rs::{FxHashSet, Jieba};

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub mod textrank;
pub mod tfidf;

pub static DEFAULT_STOP_WORDS: OnceLock<BTreeSet<String>> = OnceLock::new();
static DEFAULT_STOP_WORDS_LOOKUP: OnceLock<FxHashSet<String>> = OnceLock::new();

fn default_stop_words() -> &'static BTreeSet<String> {
    DEFAULT_STOP_WORDS.get_or_init(|| {
        BTreeSet::from_iter(
            [
                "the", "of", "is", "and", "to", "in", "that", "we", "for", "an", "are", "by", "be",
                "as", "on", "with", "can", "if", "from", "which", "you", "it", "this", "then",
                "at", "have", "all", "not", "one", "has", "or",
            ]
            .into_iter()
            .map(ToString::to_string),
        )
    })
}

fn stop_words_lookup(stop_words: &BTreeSet<String>) -> FxHashSet<String> {
    let mut lookup =
        FxHashSet::with_capacity_and_hasher(stop_words.len(), crate::rustc_hash::FxBuildHasher);
    lookup.extend(stop_words.iter().cloned());
    lookup
}

/// One weighted keyword returned by the `OpenCC::keyword_weight_*` APIs.
///
/// Available with the default-enabled `keywords` feature. Weights are specific
/// to the selected algorithm and input; they are not probabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyword {
    /// Extracted token text.
    pub keyword: String,
    /// TF-IDF or TextRank score.
    pub weight: f64,
    /// Part-of-speech tag assigned during extraction.
    pub tag: String,
}

/// Creates a KeywordExtractConfig state that contains filter criteria as well as segmentation
/// configuration for use by keyword extraction implementations.
///
/// Use [`KeywordExtractConfigBuilder`] to change the defaults.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordExtractConfig {
    stop_words: Cow<'static, BTreeSet<String>>,
    stop_words_lookup: Cow<'static, FxHashSet<String>>,
    min_keyword_length: usize,
    use_hmm: bool,
}

impl Default for KeywordExtractConfig {
    fn default() -> KeywordExtractConfig {
        KeywordExtractConfig::builder().build()
    }
}

impl KeywordExtractConfig {
    /// Creates a new [`KeywordExtractConfigBuilder`] with default values.
    pub fn builder() -> KeywordExtractConfigBuilder {
        KeywordExtractConfigBuilder::default()
    }

    /// Gets the current set of stop words.
    #[allow(dead_code)]
    pub fn stop_words(&self) -> &BTreeSet<String> {
        &self.stop_words
    }

    /// Returns whether HMM is used during segmentation in `extract_tags`.
    pub fn use_hmm(&self) -> bool {
        self.use_hmm
    }

    /// Gets the minimum number of Unicode Scalar Values required per keyword.
    pub fn min_keyword_length(&self) -> usize {
        self.min_keyword_length
    }

    #[inline]
    pub(crate) fn is_keyword(&self, s: &str) -> bool {
        let mut char_count = 0;
        let mut has_uppercase = false;
        for ch in s.chars() {
            char_count += 1;
            has_uppercase |= ch.is_uppercase();
        }

        if char_count < self.min_keyword_length() || self.stop_words_lookup.contains(s) {
            return false;
        }

        !has_uppercase || !self.stop_words_lookup.contains(&s.to_lowercase())
    }
}

#[derive(Debug, Clone)]
pub struct KeywordExtractConfigBuilder {
    stop_words: Cow<'static, BTreeSet<String>>,
    min_keyword_length: usize,
    use_hmm: bool,
}

impl Default for KeywordExtractConfigBuilder {
    fn default() -> Self {
        KeywordExtractConfigBuilder {
            stop_words: Cow::Borrowed(default_stop_words()),
            min_keyword_length: 2,
            use_hmm: false,
        }
    }
}

impl KeywordExtractConfigBuilder {
    /// Builds the [`KeywordExtractConfig`] with the current configuration.
    pub fn build(self) -> KeywordExtractConfig {
        // Unmodified defaults share both collections; custom builders own their data.
        let stop_words_lookup = match &self.stop_words {
            Cow::Borrowed(words) => {
                Cow::Borrowed(DEFAULT_STOP_WORDS_LOOKUP.get_or_init(|| stop_words_lookup(words)))
            }
            Cow::Owned(words) => Cow::Owned(stop_words_lookup(words)),
        };
        KeywordExtractConfig {
            stop_words: self.stop_words,
            stop_words_lookup,
            min_keyword_length: self.min_keyword_length,
            use_hmm: self.use_hmm,
        }
    }

    /// If set, when segment cannot be found in the dictionary, fall back to HMM model.
    #[allow(dead_code)]
    pub fn use_hmm(mut self, yes: bool) -> Self {
        self.use_hmm = yes;
        self
    }

    /// Sets the length that any segments less than it will not be considered as a keyword.
    #[allow(dead_code)]
    pub fn min_keyword_length(mut self, length: usize) -> Self {
        self.min_keyword_length = length;
        self
    }

    /// Add a new stop word.
    #[allow(dead_code)]
    pub fn add_stop_word(mut self, word: impl Into<String>) -> Self {
        self.stop_words.to_mut().insert(word.into());
        self
    }

    /// Remove a stop word.
    #[allow(dead_code)]
    pub fn remove_stop_word(mut self, word: impl AsRef<str>) -> Self {
        self.stop_words.to_mut().remove(word.as_ref());
        self
    }

    /// Replace all stop words with new stop words set.
    #[allow(dead_code)]
    pub fn set_stop_words(mut self, stop_words: BTreeSet<String>) -> Self {
        self.stop_words = Cow::Owned(stop_words);
        self
    }
}

/// Extracts keywords from a given sentence with the Jieba instance.
pub trait KeywordExtract {
    fn extract_keywords(
        &self,
        jieba: &Jieba,
        sentence: &str,
        top_k: usize,
        allowed_pos: &[&str],
    ) -> Vec<Keyword>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_share_stop_words_and_custom_builders_stay_independent() {
        let first = KeywordExtractConfig::default();
        let second = KeywordExtractConfig::builder()
            .min_keyword_length(3)
            .use_hmm(true)
            .build();
        assert!(std::ptr::eq(first.stop_words(), second.stop_words()));
        assert!(std::ptr::eq(
            first.stop_words_lookup.as_ref(),
            second.stop_words_lookup.as_ref()
        ));

        let builder = KeywordExtractConfig::builder().add_stop_word("股票");
        let custom = builder.clone().remove_stop_word("the").build();
        let other = builder.build();
        assert!(custom.is_keyword("the"));
        assert!(!custom.is_keyword("股票"));
        assert!(!other.is_keyword("the"));
        assert!(!other.is_keyword("股票"));
        assert!(!first.is_keyword("the"));
        assert!(first.is_keyword("股票"));

        let empty = KeywordExtractConfig::builder()
            .set_stop_words(BTreeSet::new())
            .build();
        assert!(empty.stop_words().is_empty());
        assert!(empty.is_keyword("the"));
        assert!(!first.stop_words().is_empty());
    }

    #[test]
    fn shared_and_owned_stop_words_filter_unicode_identically() {
        let shared = KeywordExtractConfig::default();
        let owned = KeywordExtractConfig::builder()
            .set_stop_words(shared.stop_words().clone())
            .build();
        assert_eq!(shared, owned);
        for (word, expected) in [
            ("the", false),
            ("THE", false),
            ("The", false),
            ("𠀀", false),
            ("𠀀🙂", true),
            ("股票", true),
        ] {
            assert_eq!(shared.is_keyword(word), expected, "{word}");
            assert_eq!(owned.is_keyword(word), expected, "{word}");
        }
        let custom = KeywordExtractConfig::builder()
            .set_stop_words(BTreeSet::from(["äpfel".into()]))
            .build();
        assert!(!custom.is_keyword("ÄPFEL"));
        assert!(shared.is_keyword("ÄPFEL"));
    }
}
