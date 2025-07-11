use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufWriter;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use zstd::stream::read::Decoder;
use zstd::Encoder;

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

impl Default for Dictionary {
    fn default() -> Self {
        Dictionary {
            st_characters: HashMap::new(),
            st_phrases: HashMap::new(),
            ts_characters: HashMap::new(),
            ts_phrases: HashMap::new(),
            tw_phrases: HashMap::new(),
            tw_phrases_rev: HashMap::new(),
            tw_variants: HashMap::new(),
            tw_variants_rev: HashMap::new(),
            tw_variants_rev_phrases: HashMap::new(),
            hk_variants: HashMap::new(),
            hk_variants_rev: HashMap::new(),
            hk_variants_rev_phrases: HashMap::new(),
            jps_characters: HashMap::new(),
            jps_phrases: HashMap::new(),
            jp_variants: HashMap::new(),
            jp_variants_rev: HashMap::new(),
        }
    }
}

impl Dictionary {
    pub fn new() -> Self {
        const DICTIONARY_JSON_ZSTD: &[u8] = include_bytes!("dicts/dictionary.json.zst");

        let cursor = Cursor::new(DICTIONARY_JSON_ZSTD);
        let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
        let mut json_data = String::new();
        decoder
            .read_to_string(&mut json_data)
            .expect("Failed to decompress dictionary.json");

        serde_json::from_str(&json_data).unwrap_or_else(|_| {
            eprintln!("Error: Failed to deserialize JSON data.");
            Dictionary::default()
        })
    }

    pub fn from_dicts() -> Self {
        let stc_file_str = "dicts/STCharacters.txt";
        let stp_file_str = "dicts/STPhrases.txt";
        let tsc_file_str = "dicts/TSCharacters.txt";
        let tsp_file_str = "dicts/TSPhrases.txt";
        let twp_file_str = "dicts/TWPhrases.txt";
        let twpr_file_str = "dicts/TWPhrasesRev.txt";
        let twv_file_str = "dicts/TWVariants.txt";
        let twvr_file_str = "dicts/TWVariantsRev.txt";
        let twvrp_file_str = "dicts/TWVariantsRevPhrases.txt";
        let hkv_file_str = "dicts/HKVariants.txt";
        let hkvr_file_str = "dicts/HKVariantsRev.txt";
        let hkvrp_file_str = "dicts/HKVariantsRevPhrases.txt";
        let jpsc_file_str = "dicts/JPShinjitaiCharacters.txt";
        let jpsp_file_str = "dicts/JPShinjitaiPhrases.txt";
        let jpv_file_str = "dicts/JPVariants.txt";
        let jpvr_file_str = "dicts/JPVariantsRev.txt";
        let st_characters = Dictionary::load_dictionary_from_path(stc_file_str).unwrap();
        let st_phrases = Dictionary::load_dictionary_from_path(stp_file_str).unwrap();
        let ts_characters = Dictionary::load_dictionary_from_path(tsc_file_str).unwrap();
        let ts_phrases = Dictionary::load_dictionary_from_path(tsp_file_str).unwrap();
        let tw_phrases = Dictionary::load_dictionary_from_path(twp_file_str).unwrap();
        let tw_phrases_rev = Dictionary::load_dictionary_from_path(twpr_file_str).unwrap();
        let tw_variants = Dictionary::load_dictionary_from_path(twv_file_str).unwrap();
        let tw_variants_rev = Dictionary::load_dictionary_from_path(twvr_file_str).unwrap();
        let tw_variants_rev_phrases =
            Dictionary::load_dictionary_from_path(twvrp_file_str).unwrap();
        let hk_variants = Dictionary::load_dictionary_from_path(hkv_file_str).unwrap();
        let hk_variants_rev = Dictionary::load_dictionary_from_path(hkvr_file_str).unwrap();
        let hk_variants_rev_phrases =
            Dictionary::load_dictionary_from_path(hkvrp_file_str).unwrap();
        let jps_characters = Dictionary::load_dictionary_from_path(jpsc_file_str).unwrap();
        let jps_phrases = Dictionary::load_dictionary_from_path(jpsp_file_str).unwrap();
        let jp_variants = Dictionary::load_dictionary_from_path(jpv_file_str).unwrap();
        let jp_variants_rev = Dictionary::load_dictionary_from_path(jpvr_file_str).unwrap();

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

    pub fn save_compressed(dictionary: &Dictionary, path: &str) -> Result<(), io::Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let mut encoder = Encoder::new(writer, 19)?;
        serde_json::to_writer(&mut encoder, dictionary)?;
        encoder.finish()?;
        Ok(())
    }

    // Function to serialize Dictionary to JSON and write it to a file
    pub fn serialize_to_json(&self, filename: &str) -> io::Result<()> {
        let json_string = serde_json::to_string(&self)?;
        let mut file = File::create(filename)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }
}
