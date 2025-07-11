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
// Define threshold for when to use parallel processing
const PARALLEL_THRESHOLD: usize = 1000;


pub struct OpenCC {
    pub jieba: Arc<Jieba>,
    dictionary: Dictionary,
}

impl OpenCC {
    pub fn new() -> Self {
        let dict_hans_hant_txt = decompress_jieba_dict();
        let mut dict_hans_hant = BufReader::new(dict_hans_hant_txt.as_bytes());
        let jieba = Arc::new(Jieba::with_dict(&mut dict_hans_hant).unwrap());
        let dictionary = Dictionary::new();

        OpenCC { jieba, dictionary }
    }

    fn phrases_cut_convert<'a>(
        &'a self,
        input: &'a str,
        dictionaries: &'a [&HashMap<String, String>],
        hmm: bool,
    ) -> String {
        let ranges = self.split_string_ranges(input);
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

    // Unified character conversion function
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

    // Unified string splitting function
    fn split_string_ranges(&self, text: &str) -> Vec<Range<usize>> {
        let mut ranges = Vec::new();
        let char_indices: Vec<(usize, char)> = text.char_indices().collect();

        let mut start = 0;

        for (i, &(_, ch)) in char_indices.iter().enumerate() {
            if DELIMITER_SET.contains(&ch) {
                let end = if i + 1 < char_indices.len() {
                    char_indices[i + 1].0
                } else {
                    text.len()
                };
                ranges.push(start..end);
                start = end;
            }
        }

        if start < text.len() {
            ranges.push(start..text.len());
        }

        ranges
    }

    // Unified phrases cutting function
    fn phrases_cut_impl(&self, input: &str, hmm: bool, use_parallel: bool) -> Vec<String> {
        let ranges = self.split_string_ranges(input);

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

    pub fn jieba_cut(&self, input: &str, hmm: bool) -> Vec<String> {
        let use_parallel = input.len() >= PARALLEL_THRESHOLD;
        self.phrases_cut_impl(input, hmm, use_parallel)
    }

    pub fn jieba_cut_and_join(&self, input: &str, hmm: bool, delimiter: &str) -> String {
        self.jieba_cut(input, hmm).join(delimiter)
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let result = self.phrases_cut_convert(input, &dict_refs, true);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let result = self.phrases_cut_convert(input, &dict_refs, true);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

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

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let output = Self::convert_by_char(input, &dict_refs, true);
        output
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let output = Self::convert_by_char(input, &dict_refs, true);
        output
    }

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

    fn convert_punctuation(text: &str, config: &str) -> String {
        let (regex, mapping) = if config.starts_with('s') {
            (&*S2T_REGEX, &*S2T_MAP)
        } else {
            (&*T2S_REGEX, &*T2S_MAP)
        };

        regex
            .replace_all(text, |caps: &regex::Captures| {
                let ch = caps.get(0).unwrap().as_str().chars().next().unwrap();
                // mapping.get(&ch).unwrap().to_string()
                mapping.get(&ch).copied().unwrap_or(ch).to_string()
            })
            .into_owned()
    }

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

    pub fn keyword_extract_tfidf(&self, input: &str, top_k: usize) -> Vec<String> {
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TfIdf::default();
        let top_k = keyword_extractor.extract_keywords(&self.jieba, &cleaned_input, top_k, vec![]);
        // Extract only the keyword strings from the Keyword struct
        top_k.into_iter().map(|k| k.keyword).collect()
    }

    pub fn keyword_weight_tfidf(&self, input: &str, top_k: usize) -> Vec<Keyword> {
        // Remove newline characters from the input
        let cleaned_input = input.replace(|c| c == '\n' || c == '\r', "");
        let keyword_extractor = TfIdf::default();
        let top_k = keyword_extractor.extract_keywords(&self.jieba, &cleaned_input, top_k, vec![]);

        top_k
    }
}

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

fn decompress_jieba_dict() -> String {
    let cursor = Cursor::new(DICT_HANS_HANT_ZSTD);
    let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("Failed to decompress dictionary");
    decompressed
}
