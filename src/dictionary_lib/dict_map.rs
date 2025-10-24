use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictMap {
    #[serde(default)]
    pub map: HashMap<String, String>,

    #[serde(default)]
    pub min_len: u16,
    #[serde(default)]
    pub max_len: u16,
    #[serde(default)]
    pub key_len_mask: u64,          // lengths 1..=64 → bit n-1
    #[serde(default)]
    pub long_lengths: HashSet<u16>, // >64
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
    /// Insert (k,v) and update stats *incrementally* (no rescans).
    #[inline]
    pub fn insert_with_len(&mut self, key: String, val: String, len_chars: u16) {
        // Update stats before/after insert is equivalent here since key length won’t change.
        if len_chars != 0 {
            if len_chars <= 64 {
                self.key_len_mask |= 1u64 << (len_chars - 1);
            } else {
                self.long_lengths.insert(len_chars);
            }
            if self.min_len == 0 || len_chars < self.min_len { self.min_len = len_chars; }
            if len_chars > self.max_len { self.max_len = len_chars; }
        }
        self.map.insert(key, val);
    }

    #[inline]
    pub fn get(&self, from: &str) -> Option<&str> {
        self.map.get(from).map(|s| s.as_str())
    }

    #[inline]
    pub fn has_key_len(&self, n: u16) -> bool {
        if n == 0 { return false; }
        if n <= 64 { (self.key_len_mask & (1u64 << (n - 1))) != 0 }
        else { self.long_lengths.contains(&n) }
    }
}
