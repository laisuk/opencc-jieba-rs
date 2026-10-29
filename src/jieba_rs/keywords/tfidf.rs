use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::io::{self, BufRead};

use super::ordered_float::OrderedFloat;
use std::sync::OnceLock;

use super::{Keyword, KeywordExtract, KeywordExtractConfig, KeywordExtractConfigBuilder};
use crate::jieba_rs::FxHashMap as HashMap;
use crate::jieba_rs::Jieba;

// Only built-in data is shared. A custom load detaches through Cow::to_mut;
// ordinary extraction neither copies the map nor takes a lock after first use.
static DEFAULT_IDF: OnceLock<IdfData> = OnceLock::new();
fn default_idf() -> &'static IdfData {
    DEFAULT_IDF.get_or_init(|| {
        let bytes = crate::zstd::decompress(include_bytes!("../data/idf.txt.zst"))
            .expect("failed to decompress bundled IDF");
        let text = String::from_utf8(bytes).expect("bundled IDF is not UTF-8");
        let mut data = IdfData::default();
        data.load_str(&text);
        data
    })
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapNode<'a> {
    tfidf: OrderedFloat,
    word: &'a str,
}

impl Ord for HeapNode<'_> {
    fn cmp(&self, other: &HeapNode) -> Ordering {
        other
            .tfidf
            .cmp(&self.tfidf)
            .then_with(|| self.word.cmp(other.word))
    }
}

impl PartialOrd for HeapNode<'_> {
    fn partial_cmp(&self, other: &HeapNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// TF-IDF keywords extraction
///
/// Requires the parent crate's `keywords` feature.
#[derive(Debug)]
pub struct TfIdf {
    data: Cow<'static, IdfData>,
    config: KeywordExtractConfig,
}

#[derive(Debug, Clone, Default)]
struct IdfData {
    idf_dict: HashMap<String, f64>,
    median_idf: f64,
}

/// Implementation of JiebaKeywordExtract using a TF-IDF dictionary.
///
/// This takes the segments produced by Jieba and attempts to extract keywords.
/// Segments are filtered for stopwords and short terms. They are then matched
/// against a loaded dictionary to calculate TF-IDF scores.
impl TfIdf {
    /// Creates an TfIdf.
    ///
    /// New instance with module default stop words and no initial IDF
    /// dictionary. Dictionary should be loaded later with `load_dict()` calls.
    #[allow(dead_code)] // Retained for custom IDF users of the private implementation.
    pub fn new(opt_dict: Option<&mut impl BufRead>, config: KeywordExtractConfig) -> Self {
        let mut instance = TfIdf {
            data: Cow::Owned(IdfData::default()),
            config,
        };
        if let Some(dict) = opt_dict {
            instance.load_dict(dict).unwrap();
        }
        instance
    }

    /// Merges entries from `dict` into the `idf_dict`.
    #[allow(dead_code)]
    pub fn load_dict(&mut self, dict: &mut impl BufRead) -> io::Result<()> {
        let mut buf = String::new();
        let mut idfs = Vec::new();
        while dict.read_line(&mut buf)? > 0 {
            self.data.to_mut().load_line(&buf, &mut idfs);
            buf.clear();
        }
        self.data.to_mut().set_median(idfs);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &KeywordExtractConfig {
        &self.config
    }

    #[allow(dead_code)]
    pub fn config_mut(&mut self) -> &mut KeywordExtractConfig {
        &mut self.config
    }
}

impl IdfData {
    /// Load a dictionary held in memory, line by line and without copying.
    fn load_str(&mut self, dict: &str) {
        let mut idfs = Vec::with_capacity(bytecount::count(dict.as_bytes(), b'\n') + 1);
        self.idf_dict.reserve(idfs.capacity());
        for line in dict.lines() {
            self.load_line(line, &mut idfs);
        }
        self.set_median(idfs);
    }

    /// Add the `word idf` entry on `line`, if it is one, recording the
    /// value in `idfs` as well.
    #[inline]
    fn load_line(&mut self, line: &str, idfs: &mut Vec<OrderedFloat>) {
        let mut parts = line.split_whitespace();

        let (Some(word), Some(idf)) = (
            parts.next(),
            parts.next().and_then(|x| x.parse::<f64>().ok()),
        ) else {
            return;
        };

        self.idf_dict.insert(word.to_string(), idf);
        idfs.push(OrderedFloat(idf));
    }

    /// The median of the IDFs of one load, duplicates included, so that it
    /// is over what was read rather than over the merged dictionary. It is
    /// the lower median: the value with `len / 2` values above it, which is
    /// ascending index `(len - 1) / 2`. Selection is linear, where the heap
    /// this replaced popped half the values one by one.
    fn set_median(&mut self, mut idfs: Vec<OrderedFloat>) {
        let mid = idfs.len().saturating_sub(1) / 2;
        let (_, median, _) = idfs.select_nth_unstable(mid);
        self.median_idf = median.into_inner();
    }
}

/// TF-IDF keywords extraction.
///
/// Requires the parent crate's `keywords` feature.
impl Default for TfIdf {
    /// Creates TfIdf with DEFAULT_STOP_WORDS, the default TfIdf dictionary,
    /// 2 Unicode Scalar Value minimum for keywords, and no hmm in segmentation.
    fn default() -> Self {
        TfIdf {
            data: Cow::Borrowed(default_idf()),
            config: KeywordExtractConfigBuilder::default().build(),
        }
    }
}

impl KeywordExtract for TfIdf {
    /// Uses TF-IDF algorithm to extract the `top_k` keywords from `sentence`.
    ///
    /// If `allowed_pos` is not empty, then only terms matching those parts if
    /// speech are considered.
    fn extract_keywords(
        &self,
        jieba: &Jieba,
        sentence: &str,
        top_k: usize,
        allowed_pos: Vec<String>,
    ) -> Vec<Keyword> {
        // A handful of tags at most, so a scan beats building a set.
        let allowed = |tag: &str| allowed_pos.is_empty() || allowed_pos.iter().any(|p| p == tag);

        // Per word: its frequency and the tag of its first occurrence,
        // counted as the tags come rather than from a materialized list.
        let mut term_freq: HashMap<&str, (u64, &str)> = HashMap::default();
        jieba.tag_each(sentence, self.config.use_hmm(), |t| {
            if allowed(t.tag) && self.config.is_keyword(t.word) {
                term_freq.entry(t.word).or_insert((0, t.tag)).0 += 1;
            }
        });

        if top_k == 0 {
            return Vec::new();
        }

        let total: u64 = term_freq.values().map(|(tf, _)| tf).sum();
        // The `top_k` best so far, the worst of them at the root.
        let mut heap = BinaryHeap::with_capacity(top_k.min(term_freq.len()));

        let data = self.data.as_ref();
        for (k, (tf, _)) in term_freq.iter() {
            let idf = data.idf_dict.get(*k).unwrap_or(&data.median_idf);

            let node = HeapNode {
                tfidf: OrderedFloat(*tf as f64 * idf / total as f64),
                word: k,
            };

            if heap.len() < top_k {
                heap.push(node);
            } else if let Some(mut worst) = heap.peek_mut() {
                if node < *worst {
                    *worst = node;
                }
            }
        }

        let mut res = Vec::with_capacity(heap.len());
        while let Some(w) = heap.pop() {
            res.push(Keyword {
                keyword: String::from(w.word),
                weight: w.tfidf.into_inner(),
                tag: String::from(term_freq.get(w.word).map_or("", |(_, tag)| tag)),
            });
        }

        res.reverse();
        res
    }
}

#[cfg(test)]
#[path = "tfidf_tests.rs"]
mod tests;
