use jieba_rs::{Jieba, Keyword, TfIdf};
use jieba_rs::{KeywordExtract, TextRank};
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::BufReader;
use std::io::{Cursor, Read};
use std::sync::Arc;
use zstd::stream::read::Decoder;

use crate::dictionary_lib::Dictionary;
pub mod dictionary_lib;
const DICT_HANS_HANT_ZSTD: &[u8] = include_bytes!("dictionary_lib/dicts/dict_hans_hant.txt.zst");
const DELIMITERS: &'static str = " \t\n\r!\"#$%&'()*+,-./:;<=>?@[\\]^_{}|~＝、。“”‘’『』「」﹁﹂—－（）《》〈〉？！…／＼︒︑︔︓︿﹀︹︺︙︐［﹇］﹈︕︖︰︳︴︽︾︵︶｛︷｝︸﹃﹄【︻】︼　～．，；：";
static STRIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap());
// Define threshold for when to use parallel processing
const PARALLEL_THRESHOLD: usize = 500;
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

pub struct OpenCC {
    pub jieba: Arc<Jieba>,
    dictionary: Dictionary,
}

impl OpenCC {
    pub fn new() -> Self {
        let dict_hans_hant_txt = decompress_dict();
        let mut dict_hans_hant = BufReader::new(dict_hans_hant_txt.as_bytes());
        let jieba = Arc::new(Jieba::with_dict(&mut dict_hans_hant).unwrap());
        let dictionary = Dictionary::new();

        OpenCC { jieba, dictionary }
    }

    // Generic conversion function that works with both parallel and sequential iterators
    // fn convert_by_phrases<'a, T>(
    //     phrases: impl Iterator<Item = T> + 'a,
    //     dictionaries: &'a [&HashMap<String, String>],
    //     use_parallel: bool,
    // ) -> impl Iterator<Item = String> + 'a
    // where
    //     T: AsRef<str> + Send + 'a,
    // {
    //     phrases.map(move |phrase| {
    //         let phrase_str = phrase.as_ref();
    //         for dictionary in dictionaries {
    //             if let Some(translation) = dictionary.get(phrase_str) {
    //                 return translation.clone(); // Avoid unnecessary to_string()
    //             }
    //         }
    //         Self::convert_by_char(phrase_str, dictionaries, use_parallel)
    //     })
    // }

    fn convert_by_phrases_par<'a, T>(
        phrases: impl ParallelIterator<Item = T> + 'a,
        dictionaries: &'a [&HashMap<String, String>],
    ) -> impl ParallelIterator<Item = String> + 'a
    where
        T: AsRef<str> + Send + 'a, // T must implement AsRef<str> to support both &str and String
    {
        phrases.map(move |phrase| {
            let phrase_str = phrase.as_ref(); // Convert T to &str

            // Attempt to find a full phrase match
            for dictionary in dictionaries {
                if let Some(translation) = dictionary.get(phrase_str) {
                    return translation.clone(); // Clone the String translation
                }
            }
            // If no full phrase match, perform character-by-character conversion
            Self::convert_by_char(phrase_str, dictionaries, true)
        })
    }

    // Unified character conversion function
    fn convert_by_char(
        phrase: &str,
        dictionaries: &[&HashMap<String, String>],
        use_parallel: bool,
    ) -> String {
        if use_parallel {
            phrase
                .par_chars()
                .map(|ch| Self::translate_char(ch, dictionaries))
                .collect()
        } else {
            phrase
                .chars()
                .map(|ch| Self::translate_char(ch, dictionaries))
                .collect()
        }
    }

    // Helper function for character translation logic
    fn translate_char(ch: char, dictionaries: &[&HashMap<String, String>]) -> String {
        let mut buf = [0u8; 4];
        let ch_str = ch.encode_utf8(&mut buf);
        for dictionary in dictionaries {
            if let Some(translation) = dictionary.get(ch_str) {
                return translation.clone();
            }
        }
        ch_str.to_owned()
    }
    // Unified string splitting function
    fn split_string_inclusive(&self, text: &str, use_parallel: bool) -> Vec<String> {
        let delimiters: HashSet<char> = DELIMITERS.chars().collect();

        if use_parallel {
            let collected: Vec<char> = text.par_chars().collect();
            collected
                .par_split_inclusive(|c| delimiters.contains(c))
                .map(|slice| slice.iter().collect())
                .collect()
        } else {
            let mut result = Vec::new();
            let mut current = String::new();

            for ch in text.chars() {
                current.push(ch);
                if delimiters.contains(&ch) {
                    result.push(current);
                    current = String::new();
                }
            }

            if !current.is_empty() {
                result.push(current);
            }

            result
        }
    }

    fn phrases_cut<'a>(
        &'a self,
        input: &str,
        hmm: bool,
    ) -> impl ParallelIterator<Item = String> + 'a {
        let string_chunks = self.split_string_inclusive(input, true);
        // let jieba = Arc::new(self.jieba.clone());
        self.cut_chunks_par(string_chunks, hmm)
    }

    // Helper function for parallel jieba cutting logic
    fn cut_chunks_par<'a>(
        &'a self,
        string_chunks: Vec<String>,
        hmm: bool,
    ) -> impl ParallelIterator<Item = String> + 'a {
        string_chunks
            .into_par_iter()
            .flat_map_iter(move |chunk_str| {
                self.jieba
                    .cut(&chunk_str, hmm)
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<String>>()
            })
    }

    // Unified phrases cutting function
    fn phrases_cut_impl(&self, input: &str, hmm: bool, use_parallel: bool) -> Vec<String> {
        let string_chunks = self.split_string_inclusive(input, use_parallel);

        if use_parallel {
            self.cut_chunks_par(string_chunks, hmm).collect()
        } else {
            string_chunks
                .into_iter()
                .flat_map(|chunk_str| {
                    self.jieba
                        .cut(&chunk_str, hmm)
                        .into_iter()
                        .map(str::to_owned)
                        .collect::<Vec<String>>()
                })
                .collect()
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
        let phrases = self.phrases_cut(input, true);
        // Step 3: Apply phrase and character dictionary conversion in parallel
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let converted = Self::convert_by_phrases_par(phrases, &dict_refs);
        // Step 4: Optionally apply punctuation conversion
        let result = String::from_par_iter(converted);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    // pub fn s2t(&self, input: &str, punctuation: bool) -> String {
    //     let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
    //     let use_parallel = input.len() >= PARALLEL_THRESHOLD;
    //
    //     let result = i/f use_parallel {
    //         let phrases = self.phrases_cut(input, true);
    //         let converted = Self::convert_by_phrases_par(phrases, &dict_refs);
    //         String::from_par_iter(converted)
    //     } else {
    //         let phrases = self.phrases_cut_impl(input, true, false);
    //         Self::convert_by_phrases(phrases.into_iter(), &dict_refs, use_parallel).collect()
    //     };
    //
    //     if punctuation {
    //         Self::convert_punctuation(&result, "s")
    //     } else {
    //         result
    //     }
    // }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let converted = Self::convert_by_phrases_par(phrases, &dict_refs);
        let result = String::from_par_iter(converted);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(
                Self::convert_by_phrases_par(phrases, &dict_refs),
                &dict_refs_round_2,
            ),
            &dict_refs_round_3,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let dict_refs_round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(
                Self::convert_by_phrases_par(phrases, &dict_refs),
                &dict_refs_round_2,
            ),
            &dict_refs_round_3,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "s")
        } else {
            result
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        let result = String::from_par_iter(output);
        if punctuation {
            Self::convert_punctuation(&result, "t")
        } else {
            result
        }
    }

    pub fn t2tw(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);
        String::from_par_iter(output)
    }

    pub fn t2twp(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.tw_phrases];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        String::from_par_iter(output)
    }

    pub fn tw2t(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);
        String::from_par_iter(output)
    }

    pub fn tw2tp(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases, &dict_refs),
            &dict_refs_round_2,
        );
        String::from_par_iter(output)
    }

    pub fn t2hk(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.hk_variants];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);
        String::from_par_iter(output)
    }

    pub fn hk2t(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);
        String::from_par_iter(output)
    }

    pub fn t2jp(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [&self.dictionary.jp_variants];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);

        String::from_par_iter(output)
    }

    pub fn jp2t(&self, input: &str) -> String {
        let phrases = self.phrases_cut(input, true);
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = Self::convert_by_phrases_par(phrases, &dict_refs);

        String::from_par_iter(output)
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

    // fn convert_punctuation(text: &str, config: &str) -> String {
    //     let mut s2t_punctuation_chars: HashMap<&str, &str> = HashMap::new();
    //     s2t_punctuation_chars.insert("“", "「");
    //     s2t_punctuation_chars.insert("”", "」");
    //     s2t_punctuation_chars.insert("‘", "『");
    //     s2t_punctuation_chars.insert("’", "』");
    //
    //     let t2s_punctuation_chars: HashMap<&str, &str> = s2t_punctuation_chars
    //         .iter()
    //         .map(|(&k, &v)| (v, k))
    //         .collect();
    //
    //     let mapping = if config.starts_with('s') {
    //         &s2t_punctuation_chars
    //     } else {
    //         &t2s_punctuation_chars
    //     };
    //
    //     let pattern = format!("[{}]", mapping.keys().cloned().collect::<String>());
    //     let regex = Regex::new(&pattern).unwrap();
    //
    //     regex
    //         .replace_all(text, |caps: &regex::Captures| {
    //             mapping[caps.get(0).unwrap().as_str()]
    //         })
    //         .into_owned()
    // }

    fn convert_punctuation(text: &str, config: &str) -> String {
        let (regex, mapping) = if config.starts_with('s') {
            (&*S2T_REGEX, &*S2T_MAP)
        } else {
            (&*T2S_REGEX, &*T2S_MAP)
        };

        regex
            .replace_all(text, |caps: &regex::Captures| {
                let ch = caps.get(0).unwrap().as_str().chars().next().unwrap();
                mapping.get(&ch).unwrap().to_string()
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

pub fn format_thousand(n: i32) -> String {
    let mut result_str = n.to_string();
    let mut offset = result_str.len() % 3;
    if offset == 0 {
        offset = 3;
    }

    while offset < result_str.len() {
        result_str.insert(offset, ',');
        offset += 4; // Including the added comma
    }
    result_str
}

fn decompress_dict() -> String {
    let cursor = Cursor::new(DICT_HANS_HANT_ZSTD);
    let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
    let mut decompressed = String::new();
    decoder
        .read_to_string(&mut decompressed)
        .expect("Failed to decompress dictionary");
    decompressed
}
