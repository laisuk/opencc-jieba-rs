use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{fs, io};

pub struct Dictionary {
    pub st_characters: HashMap<String, String>,
    pub st_phrases: HashMap<String, String>,
    pub ts_characters: HashMap<String, String>,
    pub ts_phrases: HashMap<String, String>,
    pub tw_phrases: HashMap<String, String>,
    pub tw_phrases_rev: HashMap<String, String>,
    pub tw_variants: HashMap<String, String>,
    pub tw_variants_rev: HashMap<String, String>,
    pub tw_variants_rev_phrases: HashMap<String, String>,
    pub hk_variants: HashMap<String, String>,
    pub hk_variants_rev: HashMap<String, String>,
    pub hk_variants_rev_phrases: HashMap<String, String>,
    pub jps_characters: HashMap<String, String>,
    pub jps_phrases: HashMap<String, String>,
    pub jp_variants: HashMap<String, String>,
    pub jp_variants_rev: HashMap<String, String>,
}

impl Dictionary {
    pub fn new() -> Self {
        let stc_file_path = include_str!("dicts/STCharacters.txt");
        let stp_file_path = include_str!("dicts/STPhrases.txt");
        let tsc_file_path = include_str!("dicts/TSCharacters.txt");
        let tsp_file_path = include_str!("dicts/TSPhrases.txt");
        let twp_file_path = include_str!("dicts/TWPhrases.txt");
        let twpr_file_path = include_str!("dicts/TWPhrasesRev.txt");
        let twv_file_path = include_str!("dicts/TWVariants.txt");
        let twvr_file_path = include_str!("dicts/TWVariantsRev.txt");
        let twvrp_file_path = include_str!("dicts/TWVariantsRevPhrases.txt");
        let hkv_file_path = include_str!("dicts/HKVariants.txt");
        let hkvr_file_path = include_str!("dicts/HKVariantsRev.txt");
        let hkvrp_file_path = include_str!("dicts/HKVariantsRevPhrases.txt");
        let jpsc_file_path = include_str!("dicts/JPShinjitaiCharacters.txt");
        let jpsp_file_path = include_str!("dicts/JPShinjitaiPhrases.txt");
        let jpv_file_path = include_str!("dicts/JPVariants.txt");
        let jpvr_file_path = include_str!("dicts/JPVariantsRev.txt");
        let stc_dict = Dictionary::load_dictionary_from_str(stc_file_path).unwrap();
        let stp_dict = Dictionary::load_dictionary_from_str(stp_file_path).unwrap();
        let tsc_dict = Dictionary::load_dictionary_from_str(tsc_file_path).unwrap();
        let tsp_dict = Dictionary::load_dictionary_from_str(tsp_file_path).unwrap();
        let twp_dict = Dictionary::load_dictionary_from_str(twp_file_path).unwrap();
        let twpr_dict = Dictionary::load_dictionary_from_str(twpr_file_path).unwrap();
        let twv_dict = Dictionary::load_dictionary_from_str(twv_file_path).unwrap();
        let twvr_dict = Dictionary::load_dictionary_from_str(twvr_file_path).unwrap();
        let twvrp_dict = Dictionary::load_dictionary_from_str(twvrp_file_path).unwrap();
        let hkv_dict = Dictionary::load_dictionary_from_str(hkv_file_path).unwrap();
        let hkvr_dict = Dictionary::load_dictionary_from_str(hkvr_file_path).unwrap();
        let hkvrp_dict = Dictionary::load_dictionary_from_str(hkvrp_file_path).unwrap();
        let jpsc_dict = Dictionary::load_dictionary_from_str(jpsc_file_path).unwrap();
        let jpsp_dict = Dictionary::load_dictionary_from_str(jpsp_file_path).unwrap();
        let jpv_dict = Dictionary::load_dictionary_from_str(jpv_file_path).unwrap();
        let jpvr_dict = Dictionary::load_dictionary_from_str(jpvr_file_path).unwrap();

        Dictionary {
            st_characters: stc_dict,
            st_phrases: stp_dict,
            ts_characters: tsc_dict,
            ts_phrases: tsp_dict,
            tw_phrases: twp_dict,
            tw_phrases_rev: twpr_dict,
            tw_variants: twv_dict,
            tw_variants_rev: twvr_dict,
            tw_variants_rev_phrases: twvrp_dict,
            hk_variants: hkv_dict,
            hk_variants_rev: hkvr_dict,
            hk_variants_rev_phrases: hkvrp_dict,
            jps_characters: jpsc_dict,
            jps_phrases: jpsp_dict,
            jp_variants: jpv_dict,
            jp_variants_rev: jpvr_dict,
        }
    }

    #[allow(dead_code)]
    fn load_dictionary_from_path<P>(filename: P) -> io::Result<HashMap<String, String>>
    where
        P: AsRef<Path>,
    {
        let file = fs::File::open(filename)?;
        let mut dictionary = HashMap::new();

        for line in BufReader::new(file).lines() {
            let line = line?;
            // let parts: Vec<&str> = line.split('\t').collect();
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok(dictionary)
    }

    fn load_dictionary_from_str(dictionary_content: &str) -> io::Result<HashMap<String, String>> {
        let mut dictionary = HashMap::new();

        for line in dictionary_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let phrase = parts[0].to_string();
                let translation = parts[1].to_string();
                dictionary.insert(phrase, translation);
            } else {
                eprintln!("Invalid line format: {}", line);
            }
        }

        Ok(dictionary)
    }
}
