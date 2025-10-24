use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a single OpenCC-style dictionary table with
/// precomputed metadata for fast phrase lookup.
///
/// Each [`DictMap`] contains a `HashMap<String, String>` mapping
/// source phrases to target phrases (e.g., Traditional → Simplified),
/// along with compact statistics such as minimum/maximum key lengths
/// and bitmask-encoded length presence for fast gating.
///
/// The structure is designed to be both **serialization-friendly**
/// (used in JSON/Zstandard artifacts) and **runtime-efficient**
/// (avoids rescanning keys on load).
///
/// # Fields
///
/// - [`map`]: The actual dictionary data mapping source → target strings.
/// - [`min_len`]: The shortest key length in Unicode scalar values.
/// - [`max_len`]: The longest key length in Unicode scalar values.
/// - [`key_len_mask`]: Bitmask (bits 0–63 → lengths 1–64) marking which
///   key lengths are present in the dictionary.
/// - [`long_lengths`]: Set of key lengths greater than 64, if any.
///
/// # Serialization
///
/// This struct is serialized/deserialized using Serde with
/// `#[serde(deny_unknown_fields)]`, ensuring strict schema validation.
/// Empty collections are serialized as `[]`, not `null`.
///
/// # Example (JSON)
///
/// ```json
/// {
///   "map": { "漢字": "汉字" },
///   "min_len": 2,
///   "max_len": 2,
///   "key_len_mask": 2,
///   "long_lengths": []
/// }
/// ```
///
/// # Notes
///
/// - Key statistics are maintained **incrementally** during text-file loading.
/// - There is no need to call a rebuild or rescan function.
/// - The bitmask allows `O(1)` checks in `has_key_len()` during segmentation.
///
/// # See Also
/// - [`Dictionary`](crate::Dictionary): Aggregates multiple [`DictMap`]s for all conversion directions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictMap {
    /// Raw mapping of source phrase → target phrase.
    #[serde(default)]
    pub map: HashMap<String, String>,

    /// Shortest phrase length in Unicode scalars.
    #[serde(default)]
    pub min_len: u16,

    /// Longest phrase length in Unicode scalars.
    #[serde(default)]
    pub max_len: u16,

    /// Bitmask (bits 0–63) for phrase lengths 1..=64.
    /// Bit *n-1* corresponds to presence of key length *n*.
    #[serde(default)]
    pub key_len_mask: u64,

    /// Set of key lengths greater than 64 (rare, but supported).
    #[serde(default)]
    pub long_lengths: HashSet<u16>,
}

impl Default for DictMap {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            min_len: 0,
            max_len: 0,
            key_len_mask: 0,
            long_lengths: HashSet::new(),
        }
    }
}

impl DictMap {
    /// Inserts a new key–value pair and updates length statistics incrementally.
    ///
    /// This method updates [`min_len`], [`max_len`], [`key_len_mask`],
    /// and [`long_lengths`] in a single pass, avoiding later rescans.
    ///
    /// # Arguments
    ///
    /// * `key` — Source phrase (UTF-8 string).
    /// * `val` — Target phrase (UTF-8 string).
    /// * `len_chars` — Number of Unicode scalar values in `key`.
    ///
    /// # Example
    ///
    /// ```
    /// use opencc_jieba_rs::dictionary_lib::DictMap;
    ///
    /// let mut d = DictMap::default();
    /// d.insert_with_len("漢字".into(), "汉字".into(), 2);
    /// assert!(d.has_key_len(2));
    /// assert_eq!(d.get("漢字"), Some("汉字"));
    /// ```
    #[inline]
    pub fn insert_with_len(&mut self, key: String, val: String, len_chars: u16) {
        if len_chars != 0 {
            if len_chars <= 64 {
                self.key_len_mask |= 1u64 << (len_chars - 1);
            } else {
                self.long_lengths.insert(len_chars);
            }
            if self.min_len == 0 || len_chars < self.min_len {
                self.min_len = len_chars;
            }
            if len_chars > self.max_len {
                self.max_len = len_chars;
            }
        }
        self.map.insert(key, val);
    }

    /// Retrieves the mapped value for a given key (if any).
    ///
    /// # Arguments
    ///
    /// * `from` — Input phrase to query.
    ///
    /// # Returns
    ///
    /// `Some(&str)` if the phrase exists, otherwise `None`.
    #[inline(always)]
    pub fn get(&self, from: &str) -> Option<&str> {
        self.map.get(from).map(|s| s.as_str())
    }

    /// Checks whether this dictionary contains any keys of a specific length.
    ///
    /// The lookup uses a 64-bit mask for lengths 1–64 and falls back
    /// to [`long_lengths`] for larger phrases.
    ///
    /// # Arguments
    ///
    /// * `n` — Phrase length in Unicode scalars.
    ///
    /// # Returns
    ///
    /// `true` if at least one key of that length exists.
    ///
    /// # Example
    ///
    /// ```
    /// use opencc_jieba_rs::dictionary_lib::DictMap;
    ///
    /// let mut d = DictMap::default();
    /// d.insert_with_len("繁體".into(), "繁体".into(), 2);
    /// assert!(d.has_key_len(2));
    /// assert!(!d.has_key_len(3));
    /// ```
    #[inline(always)]
    pub fn has_key_len(&self, n: u16) -> bool {
        if n == 0 {
            return false;
        }
        if n <= 64 {
            (self.key_len_mask & (1u64 << (n - 1))) != 0
        } else {
            self.long_lengths.contains(&n)
        }
    }
}
