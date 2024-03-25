use std::collections::HashMap;
use std::io::BufReader;

use jieba_rs::Jieba;
use regex::Regex;

use crate::zho_dictionary::Dictionary;

mod zho_dictionary;

pub struct OpenCC {
    pub jieba: Jieba,
    dictionary: Dictionary,
}

impl OpenCC {
    pub fn new() -> Self {
        let dict_hans_hant_txt = include_str!("dicts/dict_hans_hant.txt");
        let mut dict_hans_hant = BufReader::new(dict_hans_hant_txt.as_bytes());
        let jieba = Jieba::with_dict(&mut dict_hans_hant).unwrap();
        let dictionary = Dictionary::new();

        OpenCC { jieba, dictionary }
    }

    fn convert_by_slice<'a>(
        phrases: impl Iterator<Item = &'a str> + 'a,
        dictionaries: &'a [&HashMap<String, String>],
    ) -> impl Iterator<Item = String> + 'a {
        phrases.map(move |phrase| {
            // 整个词转换
            for dictionary in dictionaries {
                if let Some(translation) = dictionary.get(phrase) {
                    return translation.to_string(); // Clone the String translation
                }
            }
            // 逐字转换
            Self::convert_by_char(phrase, dictionaries)
        })
    }

    fn convert_by_string<'a>(
        phrases: impl Iterator<Item = String> + 'a,
        dictionaries: &'a [&HashMap<String, String>],
    ) -> impl Iterator<Item = String> + 'a {
        phrases.map(move |phrase| {
            // 整个词转换
            for dictionary in dictionaries {
                if let Some(translation) = dictionary.get(&phrase) {
                    return translation.to_string(); // Clone the String translation
                }
            }
            // 逐字转换
            Self::convert_by_char(&phrase, dictionaries)
        })
    }

    fn convert_by_char(phrase: &str, dictionaries: &[&HashMap<String, String>]) -> String {
        let mut phrase_builder = String::new();
        for character in phrase.chars() {
            let character_str = character.to_string();
            let mut char_found = false;
            for dictionary in dictionaries {
                if let Some(translation) = dictionary.get(&character_str) {
                    phrase_builder.push_str(translation);
                    char_found = true;
                    break;
                }
            }
            if !char_found {
                phrase_builder.push_str(&character_str);
            }
        }
        phrase_builder
    }

    pub fn s2t(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "s")
        } else {
            String::from_iter(output)
        }
    }

    pub fn t2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "t")
        } else {
            String::from_iter(output)
        }
    }

    pub fn s2tw(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "s")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn tw2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "t")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn s2twp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases];
        let dict_refs_round_3 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        let output_3 = Self::convert_by_string(output_2, &dict_refs_round_3);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_3).as_str(), "s")
        } else {
            String::from_iter(output_3)
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
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        let output_3 = Self::convert_by_string(output_2, &dict_refs_round_3);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_3).as_str(), "t")
        } else {
            String::from_iter(output_3)
        }
    }

    pub fn s2hk(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.st_phrases, &self.dictionary.st_characters];
        let dict_refs_round_2 = [&self.dictionary.hk_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "s")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn hk2s(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let dict_refs_round_2 = [&self.dictionary.ts_phrases, &self.dictionary.ts_characters];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "h")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn t2tw(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.tw_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "s")
        } else {
            String::from_iter(output)
        }
    }

    pub fn t2twp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.tw_phrases];
        let dict_refs_round_2 = [&self.dictionary.tw_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "s")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn tw2t(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "s")
        } else {
            String::from_iter(output)
        }
    }

    pub fn tw2tp(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.tw_variants_rev,
            &self.dictionary.tw_variants_rev_phrases,
        ];
        let dict_refs_round_2 = [&self.dictionary.tw_phrases_rev];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        let output_2 = Self::convert_by_string(output, &dict_refs_round_2);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output_2).as_str(), "s")
        } else {
            String::from_iter(output_2)
        }
    }

    pub fn t2hk(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.hk_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "s")
        } else {
            String::from_iter(output)
        }
    }

    pub fn hk2t(&self, input: &str, punctuation: bool) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.hk_variants_rev_phrases,
            &self.dictionary.hk_variants_rev,
        ];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);
        if punctuation {
            Self::convert_punctuation(String::from_iter(output).as_str(), "s")
        } else {
            String::from_iter(output)
        }
    }

    pub fn t2jp(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [&self.dictionary.jp_variants];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);

        String::from_iter(output)
    }

    pub fn jp2t(&self, input: &str) -> String {
        let phrases = self.jieba.cut(input, true);
        let dict_refs = [
            &self.dictionary.jps_phrases,
            &self.dictionary.jps_characters,
            &self.dictionary.jp_variants_rev,
        ];
        let output = Self::convert_by_slice(phrases.into_iter(), &dict_refs);

        String::from_iter(output)
    }

    pub fn zho_check(&self, input: &str) -> i8 {
        if input.is_empty() {
            return 0;
        }
        // let re = Regex::new(r"[[:punct:]\sA-Za-z0-9]").unwrap();
        let re = Regex::new(r"[!-/:-@\[-`{-~\t\n\v\f\r 0-9A-Za-z_]").unwrap();
        let _strip_text = re.replace_all(input, "");
        let max_bytes = find_max_utf8_length(_strip_text.as_ref(), 200);
        let strip_text = match _strip_text.len() > max_bytes {
            true => &_strip_text[..max_bytes],
            false => &_strip_text,
        };
        let code;
        if strip_text != &self.t2s(strip_text, false) {
            code = 1;
        } else {
            if strip_text != &self.s2t(strip_text, false) {
                code = 2;
            } else {
                code = 0;
            }
        }
        code
    }

    fn convert_punctuation(sv: &str, config: &str) -> String {
        let mut s2t_punctuation_chars: HashMap<&str, &str> = HashMap::new();
        s2t_punctuation_chars.insert("“", "「");
        s2t_punctuation_chars.insert("”", "」");
        s2t_punctuation_chars.insert("‘", "『");
        s2t_punctuation_chars.insert("’", "』");

        let output_text;

        if config.starts_with('s') {
            let s2t_pattern = s2t_punctuation_chars.keys().cloned().collect::<String>();
            let s2t_regex = Regex::new(&format!("[{}]", s2t_pattern)).unwrap();
            output_text = s2t_regex
                .replace_all(sv, |caps: &regex::Captures| {
                    s2t_punctuation_chars[caps.get(0).unwrap().as_str()]
                })
                .into_owned();
        } else {
            let mut t2s_punctuation_chars: HashMap<&str, &str> = HashMap::new();
            for (key, value) in s2t_punctuation_chars.iter() {
                t2s_punctuation_chars.insert(value, key);
            }
            let t2s_pattern = t2s_punctuation_chars.keys().cloned().collect::<String>();
            let t2s_regex = Regex::new(&format!("[{}]", t2s_pattern)).unwrap();
            output_text = t2s_regex
                .replace_all(sv, |caps: &regex::Captures| {
                    t2s_punctuation_chars[caps.get(0).unwrap().as_str()]
                })
                .into_owned();
        }
        output_text
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
