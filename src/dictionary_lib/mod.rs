mod dict_map;

use std::fs::File;
use std::io;
use std::io::BufWriter;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub use crate::dictionary_lib::dict_map::DictMap;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use zstd::stream::read::Decoder;
use zstd::Encoder;

pub const SCHEMA_VERSION: u16 = 2;

/// Represents a collection of various Chinese character and phrase mappings
/// used for conversion between Simplified, Traditional, Taiwanese, Hong Kong,
/// and Japanese variants.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dictionary {
    pub schema_version: u16,
    /// Simplified to Traditional character mappings.
    pub st_characters: DictMap,
    /// Simplified to Traditional phrase mappings.
    pub st_phrases: DictMap,
    /// Traditional to Simplified character mappings.
    pub ts_characters: DictMap,
    /// Traditional to Simplified phrase mappings.
    pub ts_phrases: DictMap,
    /// Taiwanese phrase mappings.
    pub tw_phrases: DictMap,
    /// Reverse Taiwanese phrase mappings.
    pub tw_phrases_rev: DictMap,
    /// Taiwanese variant mappings.
    pub tw_variants: DictMap,
    /// Reverse Taiwanese variant mappings.
    pub tw_variants_rev: DictMap,
    /// Reverse Taiwanese variant phrase mappings.
    pub tw_variants_rev_phrases: DictMap,
    /// Hong Kong variant mappings.
    pub hk_variants: DictMap,
    /// Reverse Hong Kong variant mappings.
    pub hk_variants_rev: DictMap,
    /// Reverse Hong Kong variant phrase mappings.
    pub hk_variants_rev_phrases: DictMap,
    /// Japanese Shinjitai character mappings.
    pub jps_characters: DictMap,
    /// Japanese Shinjitai phrase mappings.
    pub jps_phrases: DictMap,
    /// Japanese variant mappings.
    pub jp_variants: DictMap,
    /// Reverse Japanese variant mappings.
    pub jp_variants_rev: DictMap,
}

impl Default for Dictionary {
    /// Creates a new, empty `Dictionary` with all mappings initialized.
    fn default() -> Self {
        Dictionary {
            schema_version: 0,
            st_characters: DictMap::default(),
            st_phrases: DictMap::default(),
            ts_characters: DictMap::default(),
            ts_phrases: DictMap::default(),
            tw_phrases: DictMap::default(),
            tw_phrases_rev: DictMap::default(),
            tw_variants: DictMap::default(),
            tw_variants_rev: DictMap::default(),
            tw_variants_rev_phrases: DictMap::default(),
            hk_variants: DictMap::default(),
            hk_variants_rev: DictMap::default(),
            hk_variants_rev_phrases: DictMap::default(),
            jps_characters: DictMap::default(),
            jps_phrases: DictMap::default(),
            jp_variants: DictMap::default(),
            jp_variants_rev: DictMap::default(),
        }
    }
}

impl Dictionary {
    /// Loads the dictionary from a compressed JSON file embedded in the binary.
    ///
    /// # Panics    ///  if decompression or deserialization fails.
    pub fn new() -> Self {
        const DICTIONARY_JSON_ZSTD: &[u8] = include_bytes!("dicts/dictionary.json.zst");

        let cursor = Cursor::new(DICTIONARY_JSON_ZSTD);
        let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
        let mut json_data = String::new();
        decoder
            .read_to_string(&mut json_data)
            .expect("Failed to decompress dictionary.json");

        let dict = serde_json::from_str(&json_data).unwrap_or_else(|_| {
            eprintln!(
                "Error: Failed to deserialize JSON data. (missing fields or wrong schema version)"
            );
            Dictionary::default()
        });

        // Optional sanity check
        assert_eq!(
            dict.schema_version, SCHEMA_VERSION,
            "Unsupported dictionary schema_version"
        );

        dict
    }

    /// Loads all conversion dictionaries from raw `.txt` files in the `dicts/` directory.
    ///
    /// This method is intended for **power users** who want to build the full [`Dictionary`]
    /// structure from source text files rather than using the precompiled `.zst` versions.
    ///
    /// The following files must exist under the `dicts/` directory:
    /// - STCharacters.txt, STPhrases.txt, TSCharacters.txt, TSPhrases.txt
    /// - TWPhrases.txt, TWPhrasesRev.txt, TWVariants.txt, TWVariantsRev.txt, TWVariantsRevPhrases.txt
    /// - HKVariants.txt, HKVariantsRev.txt, HKVariantsRevPhrases.txt
    /// - JPShinjitaiCharacters.txt, JPShinjitaiPhrases.txt, JPVariants.txt, JPVariantsRev.txt
    ///
    /// # Note
    /// - These `.txt` files are **not included** in the published crate on crates.io.
    /// - To use this function, clone the repository from GitHub and ensure the `dicts/` folder is present.
    ///
    /// # Errors
    /// Panics if any file is missing or fails to load.
    /// Returns the default [`Dictionary`] instance only if `.unwrap()` is replaced by fallible handling.
    ///
    /// # Intended Use
    /// - Testing custom dictionary edits.
    /// - Regenerating runtime `.zst` dictionary packages.
    /// - Debugging dictionary mapping issues.
    ///
    /// [`Dictionary`]: Dictionary
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
        ]: [DictMap; 16] = files
            .into_iter()
            .map(|f| load(f).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Dictionary {
            schema_version: SCHEMA_VERSION,
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
    // fn load_dictionary_from_path<P>(filename: P) -> io::Result<DictMap>
    // where
    //     P: AsRef<Path>,
    // {
    //     let file = File::open(filename)?;
    //     let mut dictionary = HashMap::new();
    //
    //     for line in BufReader::new(file).lines() {
    //         let line = line?;
    //         // let parts: Vec<&str> = line.split('\t').collect();
    //         let parts: Vec<&str> = line.split_whitespace().collect();
    //         if parts.len() > 1 {
    //             let phrase = parts[0].to_string();
    //             let translation = parts[1].to_string();
    //             dictionary.insert(phrase, translation);
    //         } else {
    //             eprintln!("Invalid line format: {}", line);
    //         }
    //     }
    //
    //     Ok(dictionary)
    // }
    fn load_dictionary_from_path<P>(filename: P) -> io::Result<DictMap>
    where
        P: AsRef<Path>,
    {
        let file = File::open(filename)?;
        let mut dict = DictMap::default();

        for line in BufReader::new(file).lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let key = parts[0].to_string();
                let val = parts[1].to_string();

                // Unicode scalar count; keep consistent with the rest of your pipeline.
                let len_chars = key.chars().count() as u16;

                // Incremental stats update (no rebuild later)
                dict.insert_with_len(key, val, len_chars);
            } else if !line.trim().is_empty() {
                eprintln!("Invalid line format: {}", line);
            }
        }

        // If empty file, stats remain zeros; nothing to fix up.
        Ok(dict)
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
