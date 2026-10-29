//! Private Jieba implementation maintained as part of opencc-jieba-rs.
//!
//! Use the parent crate's `OpenCC` wrappers. This module is not an external
//! jieba-rs replacement; see VENDOR_NOTES.md for integration and provenance.
//! The parent `keywords` feature enables TF-IDF and TextRank. Segmentation
//! and POS tagging are available without it.

use std::collections::HashMap;
#[cfg(feature = "keywords")]
use std::collections::HashSet;
use std::fmt;
use std::io::BufRead;

pub(crate) type FxHashMap<K, V> = HashMap<K, V, crate::rustc_hash::FxBuildHasher>;
#[cfg(feature = "keywords")]
pub(crate) type FxHashSet<K> = HashSet<K, crate::rustc_hash::FxBuildHasher>;

pub use crate::jieba_rs::errors::Error;
pub use crate::jieba_rs::hmm::HmmModel;
#[cfg(feature = "keywords")]
pub use crate::jieba_rs::keywords::textrank::TextRank;
#[cfg(feature = "keywords")]
pub use crate::jieba_rs::keywords::tfidf::TfIdf;
#[cfg(feature = "keywords")]
pub use crate::jieba_rs::keywords::{Keyword, KeywordExtract};

mod errors;
mod hmm;
#[cfg(feature = "keywords")]
mod keywords;
mod posseg;
mod sparse_dag;
mod trie;

// Jieba::new and OpenCC share the parent's embedded Hans/Hant dictionary.

use sparse_dag::{NO_MATCH, StaticSparseDAG};
use trie::CharTrie;

/// One step of the best segmentation of a block.
#[derive(Clone, Copy)]
struct RouteEntry {
    /// Log-probability of the rest of the block from this character.
    prob: f64,
    /// Index of the character after the chosen word.
    end: usize,
    /// The chosen word's dictionary id, `NO_MATCH` for a character not in
    /// the dictionary.
    word_id: i32,
}

/// Where a token sits in the dictionary block it was cut from, so a caller
/// can consult the block's dictionary matches instead of looking words up
/// again.
struct BlockRef<'b> {
    dag: &'b StaticSparseDAG,
    chars: &'b [(u32, char)],
    /// Index in `chars` of the token's first character.
    start: usize,
}

/// Initial capacity for a result vector over `sentence`: generous for a
/// short input, so that segmenting a sentence allocates once, but bounded,
/// so that a long input with few tokens does not hand back a mostly empty
/// allocation.
fn output_capacity(sentence: &str) -> usize {
    const MAX_INITIAL_CAPACITY: usize = 1 << 12;
    (sentence.len() / 2).min(MAX_INITIAL_CAPACITY)
}

/// Per-thread buffers reused across `cut` calls, so segmenting a sentence
/// allocates only its output.
#[derive(Default)]
struct Scratch {
    route: Vec<RouteEntry>,
    dag: StaticSparseDAG,
    /// The characters of the block being segmented: byte offset within the
    /// block and value. Decoded once by the splitter and used by every
    /// later pass.
    chars: Vec<(u32, char)>,
    hmm: hmm::HmmContext,
}

/// How much of a per-thread scratch buffer is worth keeping between calls.
pub(crate) const SCRATCH_BUDGET: usize = 4 << 20;

/// Drop `buf` if it grew past `max_bytes`, so a thread that once handled a
/// huge input does not pin that memory forever.
#[inline(always)]
pub(crate) fn release_if_huge<T>(buf: &mut Vec<T>, max_bytes: usize) {
    if buf.capacity() * std::mem::size_of::<T>() > max_bytes {
        *buf = Vec::new();
    }
}

impl Scratch {
    fn release_if_huge(&mut self) {
        release_if_huge(&mut self.route, SCRATCH_BUDGET);
        release_if_huge(&mut self.chars, SCRATCH_BUDGET);
        self.dag.release_if_huge();
        self.hmm.release_if_huge();
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

/// The CJK Unified Ideographs ranges other than the main block.
#[inline]
fn is_cjk_extension(c: char) -> bool {
    matches!(c,
        '\u{3400}'..='\u{4DBF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}'
        | '\u{2A700}'..='\u{2B73F}'
        | '\u{2B740}'..='\u{2B81F}'
        | '\u{2B820}'..='\u{2CEAF}'
        | '\u{2CEB0}'..='\u{2EBEF}'
        | '\u{2F800}'..='\u{2FA1F}'
    )
}

/// The main CJK Unified Ideographs block, tested on its own before the
/// extensions: nearly every character of ordinary text falls in it, and a
/// single range check is much cheaper than the vectorized nine-range test
/// the compiler otherwise emits.
#[inline(always)]
fn is_cjk_main(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}')
}

/// Check if a character is in a CJK Unified Ideographs range.
#[inline]
fn is_cjk(c: char) -> bool {
    is_cjk_main(c) || is_cjk_extension(c)
}

/// RE_HAN_DEFAULT character class: CJK + ASCII alphanumeric + `+#&._%\-`
#[inline]
fn is_han_default(c: char) -> bool {
    if is_cjk_main(c) {
        true
    } else if c.is_ascii() {
        c.is_ascii_alphanumeric() || matches!(c, '+' | '#' | '&' | '.' | '_' | '%' | '-')
    } else {
        is_cjk_extension(c)
    }
}

/// RE_HAN_CUT_ALL character class: CJK only
#[inline]
fn is_han_cut_all(c: char) -> bool {
    is_cjk(c)
}

/// RE_SKIP_CUT_ALL: anything not in `[a-zA-Z0-9+#\n]`
#[inline]
fn is_skip_cut_all(c: char) -> bool {
    !c.is_ascii_alphanumeric() && c != '+' && c != '#' && c != '\n'
}

#[inline]
fn char_count(s: &str) -> usize {
    if s.len() >= 16 {
        bytecount::num_chars(s.as_bytes())
    } else {
        s.as_bytes().iter().filter(|&&b| (b as i8) >= -0x40).count()
    }
}

/// Iterator that splits text into matched/unmatched regions by a character classifier.
/// Matched = maximal runs where `classify(c)` is true.
/// Unmatched = everything between matched runs.
pub(crate) struct SplitByCharacterClass<'t, F> {
    text: &'t str,
    pos: usize,
    classify: F,
}

impl<'t, F: Fn(char) -> bool> SplitByCharacterClass<'t, F> {
    #[inline]
    fn new(text: &'t str, classify: F) -> Self {
        SplitByCharacterClass { text, pos: 0, classify }
    }

    /// The next run, with each of its characters recorded in `chars` as
    /// `(byte offset within the run, char)`, so later passes over the run
    /// need not decode it again.
    #[inline]
    fn next_recording(&mut self, chars: &mut Vec<(u32, char)>) -> Option<SplitState<'t>> {
        chars.clear();
        self.advance(|offset, c| chars.push((offset as u32, c)))
    }

    #[inline]
    fn advance(&mut self, mut record: impl FnMut(usize, char)) -> Option<SplitState<'t>> {
        let text = self.text;
        if self.pos >= text.len() {
            return None;
        }

        let start = self.pos;
        let mut iter = text[start..].char_indices();
        let (_, first) = iter.next().unwrap();
        let matched = (self.classify)(first);
        record(0, first);
        let mut end = start + first.len_utf8();
        for (i, c) in iter {
            if (self.classify)(c) != matched {
                break;
            }
            record(i, c);
            end = start + i + c.len_utf8();
        }
        self.pos = end;
        let run = &text[start..end];
        Some(if matched {
            SplitState::Matched(run)
        } else {
            SplitState::Unmatched(run)
        })
    }
}

impl<'t, F: Fn(char) -> bool> Iterator for SplitByCharacterClass<'t, F> {
    type Item = SplitState<'t>;

    fn next(&mut self) -> Option<SplitState<'t>> {
        self.advance(|_, _| {})
    }
}

#[derive(Debug)]
pub(crate) enum SplitState<'t> {
    Unmatched(&'t str),
    Matched(&'t str),
}

#[cfg(test)]
impl<'t> SplitState<'t> {
    #[allow(dead_code)]
    fn as_str(&self) -> &'t str {
        match self {
            SplitState::Unmatched(t) => t,
            SplitState::Matched(t) => t,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizeMode {
    /// Default mode
    Default,
    /// Search mode
    Search,
}

/// A Token
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token<'a> {
    /// Word of the token
    pub word: &'a str,
    /// Unicode start position of the token
    pub start: usize,
    /// Unicode end position of the token
    pub end: usize,
    /// Byte start position of the token in the original input
    pub byte_start: usize,
    /// Byte end position of the token in the original input
    pub byte_end: usize,
}

/// A tagged word
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Tag<'a> {
    /// Word
    pub word: &'a str,
    /// Word tag
    pub tag: &'a str,
    /// Unicode start position of the word in the original input
    pub start: usize,
    /// Unicode end position of the word in the original input
    pub end: usize,
    /// Byte start position of the word in the original input
    pub byte_start: usize,
    /// Byte end position of the word in the original input
    pub byte_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct Record {
    freq: usize,
    /// Index into `Jieba::tags`.
    tag: u32,
}

/// Jieba segmentation
#[derive(Clone)]
pub struct Jieba {
    records: Vec<Record>,
    /// `ln(freq)` of each record, kept apart from `records` so the route
    /// calculation touches a dense `f64` array rather than whole records.
    log_freqs: Vec<f64>,
    /// Distinct POS tags; records refer to them by index. A dictionary has
    /// only a few dozen tags, so this replaces one heap string per word.
    tags: Vec<Box<str>>,
    tag_ids: FxHashMap<Box<str>, u32>,
    trie: CharTrie,
    total: usize,
    log_total: f64,
    hmm_model: Option<HmmModel>,
}

impl fmt::Debug for Jieba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Jieba")
            .field("records_len", &self.records.len())
            .field("total_freq", &self.total)
            .finish()
    }
}

impl Default for Jieba {
    fn default() -> Self {
        Jieba::new()
    }
}

impl Jieba {
    /// Create a new instance with empty dict
    pub fn empty() -> Self {
        Jieba {
            records: Vec::new(),
            log_freqs: Vec::new(),
            tags: Vec::new(),
            tag_ids: FxHashMap::default(),
            trie: CharTrie::new(),
            total: 0,
            log_total: 0.0f64.ln(),
            hmm_model: None,
        }
    }

    /// Create an empty tokenizer; dictionary loading is controlled by the parent.
    // pub fn new() -> Self { Self::empty() }
    /// Creates a Jieba instance using OpenCC's embedded Chinese dictionary.
    ///
    /// The dictionary is shared with the parent OpenCC implementation.
    /// No separate `jieba.dict` is embedded.
    ///
    /// Requires the `default-dict` feature.
    pub fn new() -> Self {
        let bytes = crate::zstd::decompress(
            crate::opencc::DICT_HANS_HANT_ZSTD
        )
            .expect("failed to decompress embedded OpenCC dictionary");

        let mut reader = std::io::Cursor::new(bytes);

        Self::with_dict(&mut reader)
            .expect("failed to initialize Jieba with embedded OpenCC dictionary")
    }

    /// Create a new instance with dict
    pub fn with_dict<R: BufRead>(dict: &mut R) -> Result<Self, Error> {
        let mut instance = Self::empty();
        instance.load_dict(dict)?;
        Ok(instance)
    }



    /// Set a custom HMM model for segmentation.
    ///
    /// When set, the custom model is used instead of the compile-time embedded model
    /// for HMM-based segmentation of out-of-vocabulary words.
    ///
    /// The model can be trained using `scripts/train_hmm.py`.
    pub fn set_hmm_model(&mut self, model: HmmModel) {
        self.hmm_model = Some(model);
    }

    /// Clears all data
    ///
    /// This method performs the following actions:
    /// 1. Clears the `records` list, removing all entries.
    /// 2. Resets the dictionary trie to an empty one.
    /// 3. Sets `total` to 0, resetting the count.
    ///
    /// # Arguments
    ///
    /// * `&mut self` - Mutable reference to the current instance.
    pub fn clear(&mut self) {
        self.records.clear();
        self.log_freqs.clear();
        self.tags.clear();
        self.tag_ids.clear();
        self.trie = CharTrie::new();
        self.total = 0;
        self.update_log_total();
    }

    /// Add word to dict, return `freq`
    ///
    /// `freq`: if `None`, will be given by [suggest_freq](#method.suggest_freq)
    ///
    /// `tag`: if `None`, will be given `""`
    pub fn add_word(&mut self, word: &str, freq: Option<usize>, tag: Option<&str>) -> usize {
        if word.is_empty() {
            return 0;
        }
        let freq = freq.unwrap_or_else(|| self.suggest_freq(word));
        let tag = tag.unwrap_or("");

        match self.trie.get(word) {
            Some(word_id) => {
                let old_freq = self.records[word_id as usize].freq;
                self.set_freq(word_id as usize, freq);

                self.total += freq;
                self.total -= old_freq;
            }
            None => {
                self.push_record(word, freq, tag);
                self.total += freq;
            }
        };
        self.update_log_total();

        freq
    }

    #[inline]
    fn set_freq(&mut self, word_id: usize, freq: usize) {
        self.records[word_id].freq = freq;
        self.log_freqs[word_id] = (freq as f64).ln();
    }

    #[inline]
    fn intern_tag(&mut self, tag: &str) -> u32 {
        if let Some(&id) = self.tag_ids.get(tag) {
            return id;
        }
        let id = self.tags.len() as u32;
        self.tags.push(tag.into());
        self.tag_ids.insert(tag.into(), id);
        id
    }

    /// Append a word the dictionary does not contain yet.
    #[inline]
    fn push_record(&mut self, word: &str, freq: usize, tag: &str) {
        let word_id = self.records.len() as i32;
        let tag = self.intern_tag(tag);
        self.records.push(Record { freq, tag });
        self.log_freqs.push((freq as f64).ln());
        self.trie.insert(word, word_id);
    }

    /// Add one dictionary entry; with `check_duplicates` an existing word only
    /// has its frequency replaced.
    #[inline]
    fn load_entry(&mut self, word: &str, freq: usize, tag: &str, check_duplicates: bool) {
        if check_duplicates {
            match self.trie.get(word) {
                Some(word_id) => self.set_freq(word_id as usize, freq),
                None => self.push_record(word, freq, tag),
            }
        } else {
            self.push_record(word, freq, tag);
        }
    }

    fn finish_load(&mut self) {
        self.total = self.records.iter().map(|n| n.freq).sum();
        self.update_log_total();
    }

    /// Checks if a word exists in the dictionary.
    ///
    /// # Arguments
    ///
    /// * `word` - The word to check.
    ///
    /// # Returns
    ///
    /// * `bool` - Whether the word exists in the dictionary.
    pub fn has_word(&self, word: &str) -> bool {
        self.trie.get(word).is_some()
    }

    /// Loads a dictionary by adding entries to the existing dictionary rather than resetting it.
    ///
    /// This function reads from a `BufRead` source, parsing each line as a dictionary entry. Each entry
    /// is expected to contain a word, its frequency, and optionally a tag.
    ///
    /// # Type Parameters
    ///
    /// * `R`: A type that implements the `BufRead` trait, used for reading lines from the dictionary.
    ///
    /// # Arguments
    ///
    /// * `dict` - A mutable reference to a `BufRead` source containing the dictionary entries.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if the dictionary is successfully loaded; otherwise,
    ///   returns an error describing what went wrong.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * There is an issue reading from the provided `BufRead` source.
    /// * A line in the dictionary file contains invalid frequency data (not a valid integer).
    pub fn load_dict<R: BufRead>(&mut self, dict: &mut R) -> Result<(), Error> {
        let mut buf = String::new();
        self.total = 0;

        let mut line_no = 0;
        while dict.read_line(&mut buf)? > 0 {
            {
                line_no += 1;
                let mut iter = buf.split_whitespace();
                if let Some(word) = iter.next() {
                    let freq = iter
                        .next()
                        .map(|x| {
                            x.parse::<usize>().map_err(|e| {
                                Error::InvalidDictEntry(format!(
                                    "line {line_no} `{buf}` frequency {x} is not a valid integer: {e}"
                                ))
                            })
                        })
                        .unwrap_or(Ok(0))?;
                    let tag = iter.next().unwrap_or("");
                    self.load_entry(word, freq, tag, true);
                }
            }
            buf.clear();
        }
        self.finish_load();

        Ok(())
    }

    #[inline]
    fn update_log_total(&mut self) {
        self.log_total = (self.total as f64).ln();
    }

    fn get_word_freq(&self, word: &str, default: usize) -> usize {
        match self.trie.get(word) {
            Some(word_id) => self.records[word_id as usize].freq,
            None => default,
        }
    }

    /// Suggest word frequency to force the characters in a word to be joined or split.
    pub fn suggest_freq(&self, segment: &str) -> usize {
        let logtotal = self.log_total;
        let logfreq = self.cut(segment, false).iter().fold(0f64, |freq, token| {
            freq + (self.get_word_freq(token.word, 1) as f64).ln() - logtotal
        });
        std::cmp::max((logfreq + logtotal).exp() as usize + 1, self.get_word_freq(segment, 1))
    }

    /// Byte offset of character `idx` of `block`, or the block's length
    /// for the index past its last character.
    #[inline(always)]
    fn byte_at(block: &str, chars: &[(u32, char)], idx: usize) -> usize {
        chars.get(idx).map_or(block.len(), |&(offset, _)| offset as usize)
    }

    /// Find the best segmentation of a block along `dag`, leaving in `route`
    /// the step to take from every character.
    fn calc(&self, chars: &[(u32, char)], dag: &StaticSparseDAG, route: &mut Vec<RouteEntry>) {
        let n = chars.len();

        // Every entry read below is either written first (the loop runs from
        // the end) or is the sentinel at `n`, so stale contents from a
        // previous block are never observed and need not be cleared.
        let end_entry = |end| RouteEntry {
            prob: 0.0,
            end,
            word_id: NO_MATCH,
        };
        if n + 1 > route.len() {
            route.resize(n + 1, end_entry(0));
        }
        route[n] = end_entry(n);

        let logtotal = self.log_total;
        let log1 = 0.0f64 - logtotal; // ln(1) - logtotal, precomputed for freq=1 case
        for idx in (0..n).rev() {
            let mut best: Option<RouteEntry> = None;
            for (end, word_id) in dag.iter_edges(idx) {
                let log_freq = if word_id != NO_MATCH {
                    self.log_freqs[word_id as usize]
                } else {
                    0.0 // ln(1)
                };
                let prob = log_freq - logtotal + route[end].prob;

                if let Some(b) = best {
                    if prob > b.prob || (prob == b.prob && end > b.end) {
                        best = Some(RouteEntry { prob, end, word_id });
                    }
                } else {
                    best = Some(RouteEntry { prob, end, word_id });
                }
            }

            route[idx] = best.unwrap_or_else(|| RouteEntry {
                prob: log1 + route[idx + 1].prob,
                end: idx + 1,
                word_id: NO_MATCH,
            });
        }
    }

    fn dag(&self, chars: &[(u32, char)], dag: &mut StaticSparseDAG) {
        self.trie
            .for_each_prefix_at_every_char(chars, |char_idx, end, word_id| dag.push_edge(char_idx, end, word_id));
        dag.finish(chars.len());
    }

    /// Push every dictionary word of `block` as a token, in position order.
    fn cut_all_tokens<'a>(
        &self,
        block: &'a str,
        chars: &[(u32, char)],
        base: usize,
        block_unicode_start: usize,
        tokens: &mut Vec<Token<'a>>,
    ) {
        // The trie walk visits the words in output order, so the tokens
        // come straight from it; nothing else reads this block's matches,
        // so no DAG is built.
        let byte_offset_in_sentence = block.as_ptr() as usize - base;
        self.trie.for_each_prefix_at_every_char(chars, |idx, end, _| {
            let byte_start = chars[idx].0 as usize;
            let word = &block[byte_start..Self::byte_at(block, chars, end)];
            let bs = byte_offset_in_sentence + byte_start;
            tokens.push(Token {
                word,
                start: block_unicode_start + idx,
                end: block_unicode_start + end,
                byte_start: bs,
                byte_end: bs + word.len(),
            });
        });
    }

    /// Emit the word spanning characters `x..y` of `block` with `word_id`.
    #[inline(always)]
    fn emit_span<'a>(
        block: &'a str,
        chars: &[(u32, char)],
        x: usize,
        y: usize,
        word_id: i32,
        dag: &StaticSparseDAG,
        words: &mut impl FnMut(&'a str, i32, usize, usize, &StaticSparseDAG),
    ) {
        let word = &block[Self::byte_at(block, chars, x)..Self::byte_at(block, chars, y)];
        words(word, word_id, y - x, x, dag);
    }

    /// Segment `block` and emit its words. What the best route leaves as
    /// single characters is joined: a run of ASCII alphanumerics into one
    /// word, and with `HMM` any other run through `cut_unjoined_run`.
    fn cut_route<'a, const HMM: bool>(
        &self,
        block: &'a str,
        chars: &[(u32, char)],
        words: &mut impl FnMut(&'a str, i32, usize, usize, &StaticSparseDAG),
        route: &mut Vec<RouteEntry>,
        dag: &mut StaticSparseDAG,
        hmm_context: &mut hmm::HmmContext,
    ) {
        self.dag(chars, dag);
        self.calc(chars, dag, route);
        let n = chars.len();
        let mut x = 0;
        let mut left: Option<usize> = None;

        while x < n {
            let RouteEntry { end: y, word_id, .. } = route[x];

            if y - x == 1 && (HMM || chars[x].1.is_ascii_alphanumeric()) {
                if left.is_none() {
                    left = Some(x);
                }
            } else {
                if let Some(start) = left {
                    self.cut_run::<HMM>(block, chars, route, start, x, dag, words, hmm_context);
                    left = None;
                }
                Self::emit_span(block, chars, x, y, word_id, dag, words);
            }
            x = y;
        }

        if let Some(start) = left {
            self.cut_run::<HMM>(block, chars, route, start, n, dag, words, hmm_context);
        }

        dag.clear();
    }

    /// Emit the run of single characters the route left unjoined at
    /// characters `x..y` of `block`: without `HMM` it is a run of ASCII
    /// alphanumerics and becomes one word, whose id is known only when it
    /// is a single character the route looked up.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn cut_run<'a, const HMM: bool>(
        &self,
        block: &'a str,
        chars: &[(u32, char)],
        route: &[RouteEntry],
        x: usize,
        y: usize,
        dag: &StaticSparseDAG,
        words: &mut impl FnMut(&'a str, i32, usize, usize, &StaticSparseDAG),
        hmm_context: &mut hmm::HmmContext,
    ) {
        if HMM {
            self.cut_unjoined_run(block, chars, route, x, y, dag, words, hmm_context);
        } else {
            let id = if y - x == 1 { route[x].word_id } else { NO_MATCH };
            Self::emit_span(block, chars, x, y, id, dag, words);
        }
    }

    /// Emit the run of single characters the route left unjoined at
    /// characters `x..y` of `block`: as one word if it is in the
    /// dictionary, through the HMM if not.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn cut_unjoined_run<'a>(
        &self,
        block: &'a str,
        chars: &[(u32, char)],
        route: &[RouteEntry],
        x: usize,
        y: usize,
        dag: &StaticSparseDAG,
        words: &mut impl FnMut(&'a str, i32, usize, usize, &StaticSparseDAG),
        hmm_context: &mut hmm::HmmContext,
    ) {
        if y - x == 1 {
            Self::emit_span(block, chars, x, y, route[x].word_id, dag, words);
            return;
        }
        if dag.word_at(x, y).is_none() {
            self.hmm_cut(block, chars, x, y, dag, words, hmm_context);
        } else {
            // Each character is a route step of its own, so its id is known.
            let mut x = x;
            while x < y {
                let RouteEntry { end: next, word_id, .. } = route[x];
                Self::emit_span(block, chars, x, next, word_id, dag, words);
                x = next;
            }
        }
    }

    /// Cut characters `x..y` of `block` with the HMM.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn hmm_cut<'a>(
        &self,
        block: &'a str,
        chars: &[(u32, char)],
        x: usize,
        y: usize,
        dag: &StaticSparseDAG,
        words: &mut impl FnMut(&'a str, i32, usize, usize, &StaticSparseDAG),
        hmm_context: &mut hmm::HmmContext,
    ) {
        let run = &chars[x..y];
        let mut x = x;
        let mut words = |word: &'a str| {
            let count = char_count(word);
            words(word, NO_MATCH, count, x, dag);
            x += count;
        };
        if let Some(ref model) = self.hmm_model {
            hmm::cut_with_allocated_memory(block, run, &mut words, model, hmm_context);
        } else {
            hmm::cut_with_allocated_memory(block, run, &mut words, &hmm::builtin_hmm(), hmm_context);
        }
    }

    /// Create a Token for a word of `char_count` characters, advancing the
    /// tracked Unicode offset past it.
    #[inline]
    fn make_token_incremental<'a>(
        word: &'a str,
        base: usize,
        unicode_offset: &mut usize,
        char_count: usize,
    ) -> Token<'a> {
        let ptr = word.as_ptr() as usize;
        debug_assert!(ptr >= base, "word is not a subslice of sentence");
        let byte_start = ptr - base;
        let byte_end = byte_start + word.len();
        let start = *unicode_offset;
        *unicode_offset = start + char_count;
        Token {
            word,
            start,
            end: *unicode_offset,
            byte_start,
            byte_end,
        }
    }

    /// Segment `sentence` and hand each token to `emit` in order, along with
    /// its dictionary id when the segmentation looked the word up
    /// (`NO_MATCH` otherwise, which does not mean the word is unknown) and;
    /// for a token cut from a dictionary block, where it sits in that block.
    fn cut_each<'a>(&self, sentence: &'a str, hmm: bool, mut emit: impl FnMut(Token<'a>, i32, Option<BlockRef<'_>>)) {
        let base = sentence.as_ptr() as usize;
        let mut unicode_offset = 0;

        SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            let Scratch {
                route,
                dag,
                chars,
                hmm: hmm_context,
            } = &mut *scratch;
            let mut splitter = SplitByCharacterClass::new(sentence, is_han_default);

            while let Some(state) = splitter.next_recording(chars) {
                match state {
                    SplitState::Matched(block) => {
                        assert!(!block.is_empty());

                        let chars: &[(u32, char)] = chars;
                        let mut sink =
                            |word: &'a str, word_id: i32, char_count: usize, start: usize, dag: &StaticSparseDAG| {
                                emit(
                                    Self::make_token_incremental(word, base, &mut unicode_offset, char_count),
                                    word_id,
                                    Some(BlockRef { dag, chars, start }),
                                );
                            };
                        if hmm {
                            self.cut_route::<true>(block, chars, &mut sink, route, dag, hmm_context);
                        } else {
                            self.cut_route::<false>(block, chars, &mut sink, route, dag, hmm_context);
                        }
                    }
                    SplitState::Unmatched(block) => {
                        assert!(!block.is_empty());

                        let mut i = 0;
                        while let Some(&(offset, c)) = chars.get(i) {
                            let offset = offset as usize;
                            // Group \r\n as a single token, otherwise emit each char
                            let (word, count) = if c == '\r' && matches!(chars.get(i + 1), Some(&(_, '\n'))) {
                                (&block[offset..offset + 2], 2)
                            } else {
                                (&block[offset..offset + c.len_utf8()], 1)
                            };
                            i += count;
                            emit(
                                Self::make_token_incremental(word, base, &mut unicode_offset, count),
                                NO_MATCH,
                                None,
                            );
                        }
                    }
                }
            }
            scratch.release_if_huge();
        });
    }

    /// Cut the input text
    ///
    /// ## Params
    ///
    /// `sentence`: input text
    ///
    /// `hmm`: enable HMM or not
    pub fn cut<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<Token<'a>> {
        let mut tokens = Vec::with_capacity(output_capacity(sentence));
        self.cut_each(sentence, hmm, |token, _, _| tokens.push(token));
        tokens
    }

    /// Cut the input text, return all possible words
    ///
    /// ## Params
    ///
    /// `sentence`: input text
    pub fn cut_all<'a>(&self, sentence: &'a str) -> Vec<Token<'a>> {
        let base = sentence.as_ptr() as usize;
        let mut unicode_offset = 0;

        let mut tokens = Vec::with_capacity(output_capacity(sentence));

        SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            let Scratch { chars, .. } = &mut *scratch;
            let mut splitter = SplitByCharacterClass::new(sentence, is_han_cut_all);

            while let Some(state) = splitter.next_recording(chars) {
                match state {
                    SplitState::Matched(block) => {
                        assert!(!block.is_empty());
                        let block_unicode_start = unicode_offset;
                        unicode_offset += chars.len();
                        self.cut_all_tokens(block, chars, base, block_unicode_start, &mut tokens);
                    }
                    SplitState::Unmatched(block) => {
                        assert!(!block.is_empty());
                        // RE_SKIP_CUT_ALL matched single characters, so each
                        // is a token of its own; a run of what it did not
                        // match (alphanumerics, `+`, `#`, newlines) is one.
                        let mut i = 0;
                        while i < chars.len() {
                            let mut j = i + 1;
                            if !is_skip_cut_all(chars[i].1) {
                                while j < chars.len() && !is_skip_cut_all(chars[j].1) {
                                    j += 1;
                                }
                            }
                            let word = &block[Self::byte_at(block, chars, i)..Self::byte_at(block, chars, j)];
                            tokens.push(Self::make_token_incremental(word, base, &mut unicode_offset, j - i));
                            i = j;
                        }
                    }
                }
            }
            scratch.release_if_huge();
        });
        tokens
    }

    /// Cut the input text in search mode
    ///
    /// ## Params
    ///
    /// `sentence`: input text
    ///
    /// `hmm`: enable HMM or not
    pub fn cut_for_search<'a>(&self, sentence: &'a str, hmm: bool) -> Vec<Token<'a>> {
        let mut new_words = Vec::with_capacity(output_capacity(sentence));
        self.cut_each(sentence, hmm, |token, _, block| {
            // An alphanumeric token joined by connectors is a compound in the
            // same sense as a multi-word Chinese term, so search mode offers
            // its parts too: `WES-5.4.5` is findable as `WES` and `5.4.5` as
            // well as whole. Without this, making `cut` keep such tokens whole
            // would cost the recall the parts used to provide.
            if token.word.as_bytes().iter().any(|&b| hmm::is_skip_connector(b)) {
                let mut offset = 0;
                for part in token.word.split(hmm::is_skip_connector_char) {
                    // Only parts that could be searched on their own: a purely
                    // numeric fragment of a version or part number (the `6` of
                    // `3.6.3`) is noise, and so is a single character.
                    if part.len() >= 2 && part.bytes().any(|b| b.is_ascii_alphabetic()) {
                        let byte_start = token.byte_start + offset;
                        new_words.push(Token {
                            word: part,
                            start: token.start + char_count(&token.word[..offset]),
                            end: token.start + char_count(&token.word[..offset + part.len()]),
                            byte_start,
                            byte_end: byte_start + part.len(),
                        });
                    }
                    offset += part.len() + 1;
                }
            }
            // The dictionary matches of the block the token came from
            // already say which of its 2- and 3-character substrings are
            // words, and the block's characters give their byte offsets, so
            // neither the words nor the offsets need computing again. A token
            // from outside a dictionary block is a single character (or
            // `\r\n`) and has no such substrings.
            let Some(BlockRef { dag, chars, start: x }) = block else {
                new_words.push(token);
                return;
            };
            let word = token.word;
            let char_count = token.end - token.start;
            // Byte offset within `word` of its `k`th character.
            let rel = |k: usize| {
                if k == char_count {
                    word.len()
                } else {
                    (chars[x + k].0 - chars[x].0) as usize
                }
            };
            let mut push_gram = |i: usize, len: usize| {
                if dag.word_at(x + i, x + i + len).is_some() {
                    let gram = &word[rel(i)..rel(i + len)];
                    let byte_start = token.byte_start + rel(i);
                    new_words.push(Token {
                        word: gram,
                        start: token.start + i,
                        end: token.start + i + len,
                        byte_start,
                        byte_end: byte_start + gram.len(),
                    });
                }
            };
            if char_count > 2 {
                for i in 0..char_count - 1 {
                    push_gram(i, 2);
                }
            }
            if char_count > 3 {
                for i in 0..char_count - 2 {
                    push_gram(i, 3);
                }
            }
            new_words.push(token);
        });
        new_words
    }

    /// Tokenize
    ///
    /// ## Params
    ///
    /// `sentence`: input text
    ///
    /// `mode`: tokenize mode
    ///
    /// `hmm`: enable HMM or not
    pub fn tokenize<'a>(&self, sentence: &'a str, mode: TokenizeMode, hmm: bool) -> Vec<Token<'a>> {
        match mode {
            TokenizeMode::Default => self.cut(sentence, hmm),
            TokenizeMode::Search => self.cut_for_search(sentence, hmm),
        }
    }

    /// Tag the input text
    ///
    /// ## Params
    ///
    /// `sentence`: input text
    ///
    /// `hmm`: enable HMM or not
    pub fn tag<'a>(&'a self, sentence: &'a str, hmm: bool) -> Vec<Tag<'a>> {
        let mut tags = Vec::with_capacity(output_capacity(sentence));
        self.tag_each(sentence, hmm, |tag| tags.push(tag));
        tags
    }

    /// Tag `sentence` and hand each tagged word to `emit` in order, for
    /// callers that consume the tags as they come. `emit` runs while the
    /// segmentation scratch is borrowed, so it must not segment.
    #[allow(dead_code)]
    pub(crate) fn tag_each<'a>(&'a self, sentence: &'a str, hmm: bool, mut emit: impl FnMut(Tag<'a>)) {
        self.cut_each(sentence, hmm, |token, word_id, block| {
            let word = token.word;
            let word_id = if word_id != NO_MATCH {
                Some(word_id)
            } else if let Some(BlockRef { dag, start, .. }) = block {
                dag.word_at(start, start + (token.end - token.start))
            } else {
                self.trie.get(word)
            };
            let tag = match word_id {
                Some(word_id) => &self.tags[self.records[word_id as usize].tag as usize],
                None => self.guess_tag(word),
            };
            emit(Tag {
                word,
                tag,
                start: token.start,
                end: token.end,
                byte_start: token.byte_start,
                byte_end: token.byte_end,
            });
        });
    }

    /// Guess the POS tag for an OOV word.
    ///
    /// For CJK words, uses the posseg HMM model (when available) to predict the tag.
    /// For ASCII words, uses simple heuristics (digits → "m", alpha → "eng", else → "x").
    fn guess_tag(&self, word: &str) -> &'static str {
        let mut eng = 0;
        let mut m = 0;
        for chr in word.chars() {
            if chr.is_ascii_alphanumeric() {
                eng += 1;
                if chr.is_ascii_digit() {
                    m += 1;
                }
            }
        }
        if eng > 0 {
            return if eng == m { "m" } else { "eng" };
        }

        {
            // Only use posseg HMM for words containing CJK characters
            if word.chars().any(is_cjk) {
                return posseg::guess_tag(word);
            }
        }

        "x"
    }
}

