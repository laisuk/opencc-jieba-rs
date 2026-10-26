use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::ordered_float::OrderedFloat;

use super::{Keyword, KeywordExtract, KeywordExtractConfig, KeywordExtractConfigBuilder};
use crate::jieba_rs::FxHashMap as HashMap;
use crate::jieba_rs::Jieba;

type Weight = f64;

/// The co-occurrence graph, in compressed sparse row form: the neighbours
/// of vertex `v` are `dst[offsets[v]..offsets[v + 1]]`, and each carries the
/// share of `dst`'s rank it hands to `v`, `edge weight / total weight out of
/// dst`, which is constant across the ranking sweeps.
struct StateDiagram {
    damping_factor: Weight,
    offsets: Vec<usize>,
    dst: Vec<usize>,
    share: Vec<Weight>,
}

impl StateDiagram {
    /// Build the graph of `size` vertices from undirected weighted edges.
    /// Each vertex's neighbours keep the order the edges were given in.
    fn new(size: usize, edges: &[(usize, usize, Weight)]) -> Self {
        // Counting sort by source: each undirected edge is an entry in both
        // endpoints' lists.
        let mut offsets = vec![0usize; size + 1];
        for &(u, v, _) in edges {
            offsets[u + 1] += 1;
            offsets[v + 1] += 1;
        }
        for i in 0..size {
            offsets[i + 1] += offsets[i];
        }
        let m = offsets[size];
        let mut dst = vec![0usize; m];
        let mut weight = vec![0.0; m];
        let mut next = offsets[..size].to_vec();
        for &(u, v, w) in edges {
            dst[next[u]] = v;
            weight[next[u]] = w;
            next[u] += 1;
            dst[next[v]] = u;
            weight[next[v]] = w;
            next[v] += 1;
        }

        let mut outflow = vec![0.0; size];
        for v in 0..size {
            outflow[v] = weight[offsets[v]..offsets[v + 1]].iter().sum();
        }
        let share = dst.iter().zip(&weight).map(|(&d, &w)| w / outflow[d]).collect();

        StateDiagram {
            damping_factor: 0.85,
            offsets,
            dst,
            share,
        }
    }

    fn rank(&self) -> Vec<Weight> {
        let n = self.offsets.len() - 1;
        let default_weight = 1.0 / (n as f64);

        let mut ranking_vector = vec![default_weight; n];

        for _ in 0..20 {
            for i in 0..n {
                let range = self.offsets[i]..self.offsets[i + 1];
                let s: f64 = self.dst[range.clone()]
                    .iter()
                    .zip(&self.share[range])
                    .map(|(&d, &share)| share * ranking_vector[d])
                    .sum();

                ranking_vector[i] = (1.0 - self.damping_factor) + self.damping_factor * s;
            }
        }

        ranking_vector
    }
}

/// Text rank keywords' extraction.
///
/// Requires `textrank` feature to be enabled.
#[derive(Debug)]
pub struct TextRank {
    span: usize,
    config: KeywordExtractConfig,
}

impl TextRank {
    /// Creates an TextRank.
    pub fn new(span: usize, config: KeywordExtractConfig) -> Self {
        TextRank { span, config }
    }
}

impl Default for TextRank {
    /// Creates TextRank with 5 Unicode Scalar Value spans
    fn default() -> Self {
        TextRank::new(5, KeywordExtractConfigBuilder::default().build())
    }
}

impl KeywordExtract for TextRank {
    /// Uses TextRank algorithm to extract the `top_k` keywords from `sentence`.
    ///
    /// If `allowed_pos` is not empty, then only terms matching those parts if
    /// speech are considered.
    fn extract_keywords(&self, jieba: &Jieba, sentence: &str, top_k: usize, allowed_pos: Vec<String>) -> Vec<Keyword> {
        // A handful of tags at most, so a scan beats building a set.
        let allowed = |tag: &str| allowed_pos.is_empty() || allowed_pos.iter().any(|p| p == tag);

        // Sized for the roughly one token per four bytes that Chinese text
        // segments into, about half of them candidates.
        let tokens_guess = sentence.len() / 4;
        let mut word2id: HashMap<&str, usize> =
            HashMap::with_capacity_and_hasher(tokens_guess / 2, crate::rustc_hash::FxBuildHasher);
        // Each candidate word with the tag of its first occurrence, by id.
        let mut unique_words: Vec<(&str, &str)> = Vec::with_capacity(tokens_guess / 2);
        // Per token, the id of its word if the token is a candidate. Tokens
        // that are not keep their position, so the window is over the
        // original token positions.
        let mut candidate_ids: Vec<Option<usize>> = Vec::with_capacity(tokens_guess);
        jieba.tag_each(sentence, self.config.use_hmm(), |t| {
            let id = if allowed(t.tag) && self.config.is_keyword(t.word) {
                let next_id = unique_words.len();
                Some(*word2id.entry(t.word).or_insert_with(|| {
                    unique_words.push((t.word, t.tag));
                    next_id
                }))
            } else {
                None
            };
            candidate_ids.push(id);
        });

        let mut cooccurence: HashMap<(usize, usize), usize> = HashMap::default();
        for (i, &u) in candidate_ids.iter().enumerate() {
            let Some(u) = u else { continue };

            for &v in candidate_ids.iter().take(i + self.span).skip(i + 1) {
                let Some(v) = v else { continue };
                let entry = cooccurence.entry((u, v)).or_insert(0);
                *entry += 1;
            }
        }

        if top_k == 0 {
            return Vec::new();
        }

        let edges: Vec<(usize, usize, Weight)> = cooccurence.iter().map(|(&(u, v), &c)| (u, v, c as f64)).collect();
        let ranking_vector = StateDiagram::new(unique_words.len(), &edges).rank();

        // The `top_k` best so far, the worst of them at the root.
        let mut heap = BinaryHeap::with_capacity(top_k.min(ranking_vector.len()));

        for (k, v) in ranking_vector.iter().enumerate() {
            let node = HeapNode {
                rank: OrderedFloat(v * 1e10),
                word_id: k,
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
            let (word, tag) = unique_words[w.word_id];
            res.push(Keyword {
                keyword: word.to_string(),
                weight: w.rank.into_inner(),
                tag: String::from(tag),
            });
        }

        res.reverse();
        res
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapNode {
    rank: OrderedFloat,
    word_id: usize,
}

impl Ord for HeapNode {
    fn cmp(&self, other: &HeapNode) -> Ordering {
        other
            .rank
            .cmp(&self.rank)
            .then_with(|| self.word_id.cmp(&other.word_id))
    }
}

impl PartialOrd for HeapNode {
    fn partial_cmp(&self, other: &HeapNode) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state_diagram() {
        let diagram = StateDiagram::new(10, &[]);
        assert_eq!(diagram.offsets.len(), 11);
        assert!(diagram.dst.is_empty());
    }

    #[test]
    fn test_extract_keywords_filters_invalid_candidates() {
        let jieba = Jieba::new();
        let config = KeywordExtractConfig::builder()
            .min_keyword_length(2)
            .add_stop_word("股票")
            .build();
        let extractor = TextRank::new(5, config);

        let keywords = extractor.extract_keywords(&jieba, "今天股票跌很厉害，股票又跌", 100, vec![]);

        assert!(!keywords.is_empty());
        assert!(keywords.iter().all(|keyword| keyword.keyword.chars().count() >= 2));
        assert!(keywords.iter().all(|keyword| keyword.keyword != "股票"));
    }
}
