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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DictMap {
    /// Raw mapping of source phrase → target phrase.
    #[serde(default)]
    map: HashMap<String, String>,

    /// Shortest phrase length in Unicode scalars.
    #[serde(default)]
    min_len: u16,

    /// Longest phrase length in Unicode scalars.
    #[serde(default)]
    max_len: u16,

    /// Bitmask (bits 0–63) for phrase lengths 1..=64.
    /// Bit *n-1* corresponds to presence of key length *n*.
    #[serde(default)]
    key_len_mask: u64,

    /// Set of key lengths greater than 64 (rare, but supported).
    #[serde(default)]
    long_lengths: HashSet<u16>,
}

/// Provides a zero-initialized, empty `DictMap`.
///
/// This creates a dictionary map with:
/// - an empty `map`
/// - `min_len = 0`
/// - `max_len = 0`
/// - `key_len_mask = 0`
/// - an empty `long_lengths` set
///
/// This default state is used before any dictionary data is loaded.
/// All length-related metadata (min/max length, bitmasks, long key lengths)
/// is populated later by the dictionary builder during initialization.
///
/// In this state, the `DictMap` performs no matching and serves only as a
/// placeholder or initial container.
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
    /// This method updates [`DictMap::min_len`], [`DictMap::max_len`],
    /// [`DictMap::key_len_mask`], and [`DictMap::long_lengths`] in a single pass, avoiding later rescans.
    ///
    /// # Arguments
    ///
    /// * `key` — Source phrase (UTF-8 string).
    /// * `val` — Target phrase (UTF-8 string).
    /// * `len_chars` — Number of Unicode scalar values in `key`.
    ///
    #[inline]
    #[cfg(any(feature = "dictionary-build", test))]
    pub(crate) fn insert_with_len(&mut self, key: String, val: String, len_chars: u16) {
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
    pub(crate) fn get(&self, from: &str) -> Option<&str> {
        self.map.get(from).map(|s| s.as_str())
    }

    /// Checks whether this dictionary contains any keys of a specific length.
    ///
    /// The lookup uses a 64-bit mask for lengths 1–64 and falls back
    /// to [`DictMap::long_lengths`] for larger phrases.
    ///
    /// # Arguments
    ///
    /// * `n` — Phrase length in Unicode scalars.
    ///
    /// # Returns
    ///
    /// `true` if at least one key of that length exists.
    ///
    #[inline(always)]
    pub(crate) fn has_key_len(&self, n: u16) -> bool {
        if n == 0 {
            return false;
        }
        if n <= 64 {
            (self.key_len_mask & (1u64 << (n - 1))) != 0
        } else {
            self.long_lengths.contains(&n)
        }
    }

    #[inline(always)]
    pub(crate) fn min_len(&self) -> u16 {
        self.min_len
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
