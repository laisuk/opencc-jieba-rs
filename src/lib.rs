use jieba_rs::{Jieba, Keyword, TfIdf};
use jieba_rs::{KeywordExtract, TextRank};
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::{Cursor, Read};
use zstd::stream::read::Decoder;

use crate::dictionary_lib::Dictionary;
pub mod dictionary_lib;
const DICT_HANS_HANT_ZSTD: &[u8] = include_bytes!("dictionary_lib/dicts/dict_hans_hant.txt.zst");

lazy_static! {
    static ref STRIP_REGEX: Regex = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
}

pub struct OpenCC {
    pub jieba: Jieba,
    dictionary: Dictionary,
}

impl OpenCC {
    pub fn new() -> Self {
        let dict_hans_hant_txt = decompress_dict();
        let mut dict_hans_hant = BufReader::new(dict_hans_hant_txt.as_bytes());
        let jieba = Jieba::with_dict(&mut dict_hans_hant).unwrap();
        let dictionary = Dictionary::new();

        OpenCC { jieba, dictionary }
    }

    // fn convert_by_slice<'a>(
    //     phrases: impl Iterator<Item = &'a str> + 'a,
    //     dictionaries: &'a [&HashMap<String, String>],
    // ) -> impl Iterator<Item = String> + 'a {
    //     phrases.map(move |phrase| {
    //         for dictionary in dictionaries {
    //             if let Some(translation) = dictionary.get(phrase) {
    //                 return translation.to_string(); // Clone the String translation
    //             }
    //         }
    //         Self::convert_by_char(phrase, dictionaries)
    //     })
    // }

    // fn convert_by_string<'a>(
    //     phrases: impl Iterator<Item = String> + 'a,
    //     dictionaries: &'a [&HashMap<String, String>],
    // ) -> impl Iterator<Item = String> + 'a {
    //     phrases.map(move |phrase| {
    //         // 整个词转换
    //         for dictionary in dictionaries {
    //             if let Some(translation) = dictionary.get(&phrase) {
    //                 return translation.to_string(); // Clone the String translation
    //             }
    //         }
    //         // 逐字转换
    //         Self::convert_by_char(&phrase, dictionaries)
    //     })
    // }

    // fn convert_by_char(phrase: &str, dictionaries: &[&HashMap<String, String>]) -> String {
    //     let mut phrase_builder = String::new();
    //     phrase_builder.reserve(phrase.len());
    //     for ch in phrase.chars() {
    //         let ch_str = ch.to_string();
    //         let mut char_found = false;
    //         for dictionary in dictionaries {
    //             if let Some(translation) = dictionary.get(&ch_str) {
    //                 phrase_builder.push_str(translation);
    //                 char_found = true;
    //                 break;
    //             }
    //         }
    //         if !char_found {
    //             phrase_builder.push_str(&ch_str);
    //         }
    //     }
    //     phrase_builder
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
            Self::convert_by_char_par(phrase_str, dictionaries)
        })
    }

    fn convert_by_char_par(phrase: &str, dictionaries: &[&HashMap<String, String>]) -> String {
        phrase
            .par_chars()
            .map(|ch| {
                let mut buf = [0u8; 4];
                let ch_str = ch.encode_utf8(&mut buf); // Avoid allocation
                for dictionary in dictionaries {
                    if let Some(translation) = dictionary.get(ch_str) {
                        return translation.clone(); // Found translation, return it
                    }
                }
                ch_str.to_owned() // No translation, return original character
            })
            .collect() // Collect results into a String
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "s")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "t")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "s")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "t")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(
                Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
                &dict_refs_round_2,
            ),
            &dict_refs_round_3,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "s")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn tw2sp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let dict_refs_round_3 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(
                Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
                &dict_refs_round_2,
            ),
            &dict_refs_round_3,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "t")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "s")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        if punctuation {
            Self::convert_punctuation(String::from_par_iter(output).as_str(), "h")
        } else {
            String::from_par_iter(output)
        }
    }

    pub fn t2tw(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        String::from_par_iter(output)
    }

    pub fn t2twp(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.tw_phrases];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        String::from_par_iter(output)
    }

    pub fn tw2t(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        String::from_par_iter(output)
    }

    pub fn tw2tp(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let output = Self::convert_by_phrases_par(
            Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs),
            &dict_refs_round_2,
        );
        String::from_par_iter(output)
    }

    pub fn t2hk(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.hk_variants];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        String::from_par_iter(output)
    }

    pub fn hk2t(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);
        String::from_par_iter(output)
    }

    pub fn t2jp(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.jp_variants];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);

        String::from_par_iter(output)
    }

    pub fn jp2t(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = Self::convert_by_phrases_par(phrases.into_par_iter(), &dict_refs);

        String::from_par_iter(output)
    }

    fn st(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.st_characters];
        let output = Self::convert_by_char_par(input, &dict_refs);
        output
    }

    fn ts(&self, input: &str) -> String {
        let dict_refs = [&self.dictionary.ts_characters];
        let output = Self::convert_by_char_par(input, &dict_refs);
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
        let mut s2t_punctuation_chars: HashMap<&str, &str> = HashMap::new();
        s2t_punctuation_chars.insert("“", "「");
        s2t_punctuation_chars.insert("”", "」");
        s2t_punctuation_chars.insert("‘", "『");
        s2t_punctuation_chars.insert("’", "』");

        let mapping = if config.starts_with('s') {
            &s2t_punctuation_chars
        } else {
            // Correctly create a new HashMap with reversed key-value pairs
            &s2t_punctuation_chars
                .iter()
                .map(|(&k, &v)| (v, k))
                .collect::<HashMap<&str, &str>>()
        };

        let pattern = format!("[{}]", mapping.keys().cloned().collect::<String>());
        let regex = Regex::new(&pattern).unwrap();

        regex
            .replace_all(text, |caps: &regex::Captures| {
                mapping[caps.get(0).unwrap().as_str()]
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
