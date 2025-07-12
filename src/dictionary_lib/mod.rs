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

/// Represents a collection of various Chinese character and phrase mappings
/// used for conversion between Simplified, Traditional, Taiwanese, Hong Kong,
/// and Japanese variants.
#[derive(Serialize, Deserialize)]
pub struct Dictionary {
    /// Simplified to Traditional character mappings.
    pub st_characters: HashMap<String, String>,
    /// Simplified to Traditional phrase mappings.
    pub st_phrases: HashMap<String, String>,
    /// Traditional to Simplified character mappings.
    pub ts_characters: HashMap<String, String>,
    /// Traditional to Simplified phrase mappings.
    pub ts_phrases: HashMap<String, String>,
    /// Taiwanese phrase mappings.
    pub tw_phrases: HashMap<String, String>,
    /// Reverse Taiwanese phrase mappings.
    pub tw_phrases_rev: HashMap<String, String>,
    /// Taiwanese variant mappings.
    pub tw_variants: HashMap<String, String>,
    /// Reverse Taiwanese variant mappings.
    pub tw_variants_rev: HashMap<String, String>,
    /// Reverse Taiwanese variant phrase mappings.
    pub tw_variants_rev_phrases: HashMap<String, String>,
    /// Hong Kong variant mappings.
    pub hk_variants: HashMap<String, String>,
    /// Reverse Hong Kong variant mappings.
    pub hk_variants_rev: HashMap<String, String>,
    /// Reverse Hong Kong variant phrase mappings.
    pub hk_variants_rev_phrases: HashMap<String, String>,
    /// Japanese Shinjitai character mappings.
    pub jps_characters: HashMap<String, String>,
    /// Japanese Shinjitai phrase mappings.
    pub jps_phrases: HashMap<String, String>,
    /// Japanese variant mappings.
    pub jp_variants: HashMap<String, String>,
    /// Reverse Japanese variant mappings.
    pub jp_variants_rev: HashMap<String, String>,
}

impl Default for Dictionary {
    /// Creates a new, empty `Dictionary` with all mappings initialized.
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
    /// Loads the dictionary from a compressed JSON file embedded in the binary.
    ///
    /// # Panics
    /// Panics if decompression or deserialization fails.
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

    /// Loads the dictionary from individual text files in the `dicts` directory.
    ///
    /// # Errors
    /// Returns the default dictionary if any file fails to load or parse.
    pub fn from_dicts() -> Self {
        let load = Self::load_dictionary_from_path;

        let files = [
            "dicts/STCharacters.txt",
            "dicts/STPhrases.txt",
            "dicts/TSCharacters.txt",
            "dicts/TSPhrases.txt",
            "dicts/TWPhrases.txt",
            "dicts/TWPhrasesRev.txt",
            "dicts/TWVariants.txt",
            "dicts/TWVariantsRev.txt",
            "dicts/TWVariantsRevPhrases.txt",
            "dicts/HKVariants.txt",
            "dicts/HKVariantsRev.txt",
            "dicts/HKVariantsRevPhrases.txt",
            "dicts/JPShinjitaiCharacters.txt",
            "dicts/JPShinjitaiPhrases.txt",
            "dicts/JPVariants.txt",
            "dicts/JPVariantsRev.txt",
        ];

        let [
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
        ]: [HashMap<String, String>; 16] = files
            .into_iter()
            .map(|f| load(f).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

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

    /// Loads a dictionary mapping from a file at the given path.
    ///
    /// Each line should contain a phrase and its translation separated by whitespace.
    ///
    /// # Errors
    /// Returns an `io::Error` if the file cannot be read.
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

    /// Saves the dictionary to a file in compressed JSON format using Zstandard.
    ///
    /// # Arguments
    /// * `dictionary` - The dictionary to save.
    /// * `path` - The file path to write to.
    ///
    /// # Errors
    /// Returns an `io::Error` if writing fails.
    pub fn save_compressed(dictionary: &Dictionary, path: &str) -> Result<(), io::Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let mut encoder = Encoder::new(writer, 19)?;
        serde_json::to_writer(&mut encoder, dictionary)?;
        encoder.finish()?;
        Ok(())
    }

    /// Serializes the dictionary to a JSON file.
    ///
    /// # Arguments
    /// * `filename` - The file path to write to.
    ///
    /// # Errors
    /// Returns an `io::Error` if writing fails.
    pub fn serialize_to_json(&self, filename: &str) -> io::Result<()> {
        let json_string = serde_json::to_string(&self)?;
        let mut file = File::create(filename)?;
        file.write_all(json_string.as_bytes())?;
        Ok(())
    }
}
