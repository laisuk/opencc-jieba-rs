use std::collections::HashMap;
use std::{fs, io};
use std::io::{BufRead, BufReader};
use std::path::Path;

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
    pub jp_variants_rev: HashMap<String, String>
}

impl Dictionary {
    pub fn new() -> Self {
        let stc_filename = "src/dicts/STCharacters.txt";
        let stp_filename = "src/dicts/STPhrases.txt";
        let tsc_filename = "src/dicts/TSCharacters.txt";
        let tsp_filename = "src/dicts/TSPhrases.txt";
        let twp_filename = "src/dicts/TWPhrases.txt";
        let twpr_filename = "src/dicts/TWPhrasesRev.txt";
        let twv_filename = "src/dicts/TWVariants.txt";
        let twvr_filename = "src/dicts/TWVariantsRev.txt";
        let twvrp_filename = "src/dicts/TWVariantsRevPhrases.txt";
        let hkv_filename = "src/dicts/HKVariants.txt";
        let hkvr_filename = "src/dicts/HKVariantsRev.txt";
        let hkvrp_filename = "src/dicts/HKVariantsRevPhrases.txt";
        let jpsc_filename = "src/dicts/JPShinjitaiCharacters.txt";
        let jpsp_filename = "src/dicts/JPShinjitaiPhrases.txt";
        let jpv_filename = "src/dicts/JPVariants.txt";
        let jpvr_filename = "src/dicts/JPVariantsRev.txt";
        let stc_dict = Dictionary::load_dictionary(stc_filename).unwrap();
        let stp_dict = Dictionary::load_dictionary(stp_filename).unwrap();
        let tsc_dict = Dictionary::load_dictionary(tsc_filename).unwrap();
        let tsp_dict = Dictionary::load_dictionary(tsp_filename).unwrap();
        let twp_dict = Dictionary::load_dictionary(twp_filename).unwrap();
        let twpr_dict = Dictionary::load_dictionary(twpr_filename).unwrap();
        let twv_dict = Dictionary::load_dictionary(twv_filename).unwrap();
        let twvr_dict = Dictionary::load_dictionary(twvr_filename).unwrap();
        let twvrp_dict = Dictionary::load_dictionary(twvrp_filename).unwrap();
        let hkv_dict = Dictionary::load_dictionary(hkv_filename).unwrap();
        let hkvr_dict = Dictionary::load_dictionary(hkvr_filename).unwrap();
        let hkvrp_dict = Dictionary::load_dictionary(hkvrp_filename).unwrap();
        let jpsc_dict = Dictionary::load_dictionary(jpsc_filename).unwrap();
        let jpsp_dict = Dictionary::load_dictionary(jpsp_filename).unwrap();
        let jpv_dict = Dictionary::load_dictionary(jpv_filename).unwrap();
        let jpvr_dict = Dictionary::load_dictionary(jpvr_filename).unwrap();

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
            jp_variants_rev: jpvr_dict
        }
    }

    fn load_dictionary<P>(filename: P) -> io::Result<HashMap<String, String>>
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
}