use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{fs, io};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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
        let json_data = include_str!("dicts/dictionary.json");
        let dictionary: Dictionary = serde_json::from_str(json_data).unwrap();
        dictionary
    }

    pub fn from_dicts() -> Self {
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
        let st_characters = Dictionary::load_dictionary_from_str(stc_file_path).unwrap();
        let st_phrases = Dictionary::load_dictionary_from_str(stp_file_path).unwrap();
        let ts_characters = Dictionary::load_dictionary_from_str(tsc_file_path).unwrap();
        let ts_phrases = Dictionary::load_dictionary_from_str(tsp_file_path).unwrap();
        let tw_phrases = Dictionary::load_dictionary_from_str(twp_file_path).unwrap();
        let tw_phrases_rev =
            Dictionary::load_dictionary_from_str(twpr_file_path).unwrap();
        let tw_variants = Dictionary::load_dictionary_from_str(twv_file_path).unwrap();
        let tw_variants_rev =
            Dictionary::load_dictionary_from_str(twvr_file_path).unwrap();
        let tw_variants_rev_phrases =
            Dictionary::load_dictionary_from_str(twvrp_file_path).unwrap();
        let hk_variants = Dictionary::load_dictionary_from_str(hkv_file_path).unwrap();
        let hk_variants_rev =
            Dictionary::load_dictionary_from_str(hkvr_file_path).unwrap();
        let hk_variants_rev_phrases =
            Dictionary::load_dictionary_from_str(hkvrp_file_path).unwrap();
        let jps_characters =
            Dictionary::load_dictionary_from_str(jpsc_file_path).unwrap();
        let jps_phrases = Dictionary::load_dictionary_from_str(jpsp_file_path).unwrap();
        let jp_variants = Dictionary::load_dictionary_from_str(jpv_file_path).unwrap();
        let jp_variants_rev =
            Dictionary::load_dictionary_from_str(jpvr_file_path).unwrap();

        Dictionary {
            st_characters,
            st_phrases,
            ts_characters,
            ts_phrases,
            tw_phrases,
            tw_phrases_rev,
            tw_variants,
            tw_variants_rev,
            tw_variants_rev_phrases,
            hk_variants,
            hk_variants_rev,
            hk_variants_rev_phrases,
            jps_characters,
            jps_phrases,
            jp_variants,
            jp_variants_rev,
        }
    }
    #[allow(dead_code)]
    pub fn from_json_file(filename: &str) -> io::Result<Self> {
        // Read the contents of the JSON file
        let json_string = fs::read_to_string(filename)?;
        // Deserialize the JSON string into a Dictionary struct
        let dictionary: Dictionary = serde_json::from_str(&json_string)?;

        Ok(dictionary)
    }

    #[allow(dead_code)]
    fn load_dictionary_from_path<P>(filename: P) -> io::Result<HashMap<String, String>>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    // Function to serialize Dictionary to JSON and write it to a file
    pub fn serialize_to_json(&self, filename: &str) -> io::Result<()> {
        let json_string = serde_json::to_string(&self)?;
        let mut file = File::create(filename)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }
}
