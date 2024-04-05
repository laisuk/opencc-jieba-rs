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

    #[allow(dead_code)]
    pub fn from_json(filename: &str) -> io::Result<Self> {
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
