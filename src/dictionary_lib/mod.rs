mod dict_map;

#[cfg(feature = "dictionary-build")]
use std::fs::File;
#[cfg(feature = "dictionary-build")]
use std::io;
#[cfg(feature = "dictionary-build")]
use std::io::{BufRead, BufReader, BufWriter, Write};
#[cfg(feature = "dictionary-build")]
use std::path::Path;

pub(crate) use crate::dictionary_lib::dict_map::DictMap;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read};
use zstd::stream::read::Decoder;
#[cfg(feature = "dictionary-build")]
use zstd::Encoder;

pub(crate) const SCHEMA_VERSION: u16 = 3;

/// Represents a collection of various Chinese character and phrase mappings
/// used for conversion between Simplified, Traditional, Taiwanese, Hong Kong,
/// and Japanese variants.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Dictionary {
    pub(crate) schema_version: u16,
    /// Simplified to Traditional character mappings.
    pub(crate) st_characters: DictMap,
    /// Simplified to Traditional phrase mappings.
    pub(crate) st_phrases: DictMap,
    /// Traditional to Simplified character mappings.
    pub(crate) ts_characters: DictMap,
    /// Traditional to Simplified phrase mappings.
    pub(crate) ts_phrases: DictMap,
    /// Taiwanese phrase mappings.
    pub(crate) tw_phrases: DictMap,
    /// Reverse Taiwanese phrase mappings.
    pub(crate) tw_phrases_rev: DictMap,
    /// Hong Kong phrase mappings.
    #[serde(default)]
    pub(crate) hk_phrases: DictMap,
    /// Reverse Hong Kong phrase mappings.
    #[serde(default)]
    pub(crate) hk_phrases_rev: DictMap,
    /// Taiwanese variant phrase mappings.
    #[serde(default)]
    pub(crate) tw_variants_phrases: DictMap,
    /// Taiwanese variant mappings.
    pub(crate) tw_variants: DictMap,
    /// Reverse Taiwanese variant mappings.
    pub(crate) tw_variants_rev: DictMap,
    /// Reverse Taiwanese variant phrase mappings.
    pub(crate) tw_variants_rev_phrases: DictMap,
    /// Hong Kong variant phrase mappings.
    #[serde(default)]
    pub(crate) hk_variants_phrases: DictMap,
    /// Hong Kong variant mappings.
    pub(crate) hk_variants: DictMap,
    /// Reverse Hong Kong variant mappings.
    pub(crate) hk_variants_rev: DictMap,
    /// Reverse Hong Kong variant phrase mappings.
    pub(crate) hk_variants_rev_phrases: DictMap,
    /// Japanese Shinjitai character mappings.
    pub(crate) jps_characters: DictMap,
    /// Reverse Japanese Shinjitai character mappings.
    #[serde(default)]
    pub(crate) jps_characters_rev: DictMap,
    /// Japanese Shinjitai phrase mappings.
    pub(crate) jps_phrases: DictMap,
    /// Legacy schema-2 Traditional-to-Japanese mappings.
    #[serde(default, rename = "jp_variants", skip_serializing)]
    pub(crate) legacy_jp_variants: DictMap,
    /// Legacy schema-2 Japanese-to-Traditional mappings.
    #[serde(default, rename = "jp_variants_rev", skip_serializing)]
    pub(crate) legacy_jp_variants_rev: DictMap,
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
            hk_phrases: DictMap::default(),
            hk_phrases_rev: DictMap::default(),
            tw_variants_phrases: DictMap::default(),
            tw_variants: DictMap::default(),
            tw_variants_rev: DictMap::default(),
            tw_variants_rev_phrases: DictMap::default(),
            hk_variants_phrases: DictMap::default(),
            hk_variants: DictMap::default(),
            hk_variants_rev: DictMap::default(),
            hk_variants_rev_phrases: DictMap::default(),
            jps_characters: DictMap::default(),
            jps_characters_rev: DictMap::default(),
            jps_phrases: DictMap::default(),
            legacy_jp_variants: DictMap::default(),
            legacy_jp_variants_rev: DictMap::default(),
        }
    }
}

impl Dictionary {
    /// Loads the built-in dictionary from an embedded, Zstd-compressed JSON file.
    ///
    /// This constructor reads `dictionary.json.zst` bundled at compile time via
    /// [`include_bytes!`], decompresses it using Zstd, and deserializes it into
    /// a [`Dictionary`] structure.
    ///
    /// # Behavior
    ///
    /// - The dictionary is loaded **entirely in memory** from a baked-in byte slice.
    /// - This method is intended for applications that ship with a fixed dictionary
    ///   set and do not require external configuration.
    /// - If deserialization fails due to missing fields or a schema mismatch,
    ///   a default, empty [`Dictionary`] is returned and a diagnostic message is
    ///   printed to stderr.
    ///
    /// After loading, the method also performs a schema compatibility check and
    /// panics if the embedded data uses an unsupported `schema_version`.
    ///
    /// # Panics
    ///
    /// This function will panic in the following situations:
    ///
    /// - If the Zstd decoder cannot be created.
    /// - If decompression of `dictionary.json.zst` fails.
    /// - If the loaded dictionary's `schema_version` does not match the crate's
    ///   expected [`SCHEMA_VERSION`].
    ///
    pub(crate) fn new() -> Self {
        const DICTIONARY_JSON_ZSTD: &[u8] = include_bytes!("dicts/dictionary.json.zst");

        let cursor = Cursor::new(DICTIONARY_JSON_ZSTD);
        let mut decoder = Decoder::new(cursor).expect("Failed to create zstd decoder");
        let mut json_data = String::new();
        decoder
            .read_to_string(&mut json_data)
            .expect("Failed to decompress dictionary.json");

        let dict: Dictionary = serde_json::from_str(&json_data).unwrap_or_else(|e| {
            eprintln!("Error: Failed to deserialize dictionary JSON: {e}");
            Dictionary::default()
        });

        // Optional sanity check
        assert!(
            dict.schema_version <= SCHEMA_VERSION,
            "Unsupported future dictionary schema_version"
        );

        dict
    }

    /// Loads all conversion dictionaries from raw `.txt` files in the `dicts/` directory.
    ///
    /// This method is used by internal workspace tooling to build the full
    /// dictionary structure from source text files.
    ///
    /// The following files must exist under the `dicts/` directory:
    /// - STCharacters.txt, STPhrases.txt, TSCharacters.txt, TSPhrases.txt
    /// - TWPhrases.txt, TWPhrasesRev.txt, TWVariantsPhrases.txt, TWVariants.txt, TWVariantsRev.txt, TWVariantsRevPhrases.txt
    /// - HKPhrases.txt, HKPhrasesRev.txt, HKVariantsPhrases.txt, HKVariants.txt, HKVariantsRev.txt, HKVariantsRevPhrases.txt
    /// - JPShinjitaiCharacters.txt, JPShinjitaiCharactersRev.txt, JPShinjitaiPhrases.txt
    ///
    /// # Note
    /// - These `.txt` files are **not included** in the published crate on crates.io.
    /// - To use this function, clone the repository from GitHub and ensure the `dicts/` folder is present.
    ///
    /// # Errors
    /// Panics if any file is missing or fails to load.
    /// Returns the default [`Dictionary`] instance only if `.unwrap()` is replaced by fallible handling.
    ///
    #[cfg(feature = "dictionary-build")]
    pub(crate) fn from_dicts() -> Self {
        let load = Self::load_dictionary_from_path;

        let files = [
            "dicts/STCharacters.txt",
            "dicts/STPhrases.txt",
            "dicts/TSCharacters.txt",
            "dicts/TSPhrases.txt",
            "dicts/TWPhrases.txt",
            "dicts/TWPhrasesRev.txt",
            "dicts/HKPhrases.txt",
            "dicts/HKPhrasesRev.txt",
            "dicts/TWVariantsPhrases.txt",
            "dicts/TWVariants.txt",
            "dicts/TWVariantsRev.txt",
            "dicts/TWVariantsRevPhrases.txt",
            "dicts/HKVariantsPhrases.txt",
            "dicts/HKVariants.txt",
            "dicts/HKVariantsRev.txt",
            "dicts/HKVariantsRevPhrases.txt",
            "dicts/JPShinjitaiCharacters.txt",
            "dicts/JPShinjitaiCharactersRev.txt",
            "dicts/JPShinjitaiPhrases.txt",
        ];

        let [
        st_characters,
        st_phrases,
        ts_characters,
        ts_phrases,
        tw_phrases,
        tw_phrases_rev,
        hk_phrases,
        hk_phrases_rev,
        tw_variants_phrases,
        tw_variants,
        tw_variants_rev,
        tw_variants_rev_phrases,
        hk_variants_phrases,
        hk_variants,
        hk_variants_rev,
        hk_variants_rev_phrases,
        jps_characters,
        jps_characters_rev,
        jps_phrases,
        ]: [DictMap; 19] = files
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
            hk_phrases,
            hk_phrases_rev,
            tw_variants_phrases,
            tw_variants,
            tw_variants_rev,
            tw_variants_rev_phrases,
            hk_variants_phrases,
            hk_variants,
            hk_variants_rev,
            hk_variants_rev_phrases,
            jps_characters,
            jps_characters_rev,
            jps_phrases,
            legacy_jp_variants: DictMap::default(),
            legacy_jp_variants_rev: DictMap::default(),
        }
    }

    /// Loads an OpenCC dictionary file.
    ///
    /// Rules:
    /// - Skip blank lines
    /// - Skip comment/header lines starting with `#` (after trimming leading whitespace)
    /// - Strip UTF-8 BOM (`\u{FEFF}`) on the first *data* line
    /// - Parse strictly `key<TAB>value(s)`
    /// - If a line is malformed, warn and skip *only that line*
    #[cfg(feature = "dictionary-build")]
    fn load_dictionary_from_path<P>(filename: P) -> io::Result<DictMap>
    where
        P: AsRef<Path>,
    {
        let path = filename.as_ref();
        let file = File::open(path)?;
        let mut dict = DictMap::default();

        let mut saw_data_line = false;

        for (lineno, line_res) in BufReader::new(file).lines().enumerate() {
            let line = line_res?;
            let mut s = line.trim_end();

            if s.is_empty() {
                continue;
            }

            if s.trim_start().starts_with('#') {
                continue;
            }

            if !saw_data_line {
                if let Some(rest) = s.strip_prefix('\u{FEFF}') {
                    s = rest;
                }
                saw_data_line = true;

                if s.is_empty() {
                    continue;
                }
            }

            let Some((k, v)) = s.split_once('\t') else {
                eprintln!(
                    "[warn] {}:{} skipped: missing TAB separator: {}",
                    path.display(),
                    lineno + 1,
                    s
                );
                continue;
            };

            // Keep first candidate if multiple values exist.
            let val = v.split_whitespace().next().unwrap_or("");

            if k.is_empty() || val.is_empty() {
                eprintln!(
                    "[warn] {}:{} skipped: empty key/value: {}",
                    path.display(),
                    lineno + 1,
                    s
                );
                continue;
            }

            let key = k.to_string();
            let val = val.to_string();
            let len_chars = key.chars().count() as u16;

            dict.insert_with_len(key, val, len_chars);
        }

        Ok(dict)
    }

    /// Saves this dictionary as compressed JSON using Zstandard.
    #[cfg(feature = "dictionary-build")]
    pub(crate) fn save_json_compressed(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let mut encoder = Encoder::new(writer, 19)?;
        serde_json::to_writer(&mut encoder, self)?;
        encoder.finish()?;
        Ok(())
    }

    /// Saves this dictionary as compact or pretty-printed JSON.
    #[cfg(feature = "dictionary-build")]
    pub(crate) fn save_json(&self, path: impl AsRef<Path>, pretty: bool) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        if pretty {
            serde_json::to_writer_pretty(&mut writer, self)?;
        } else {
            serde_json::to_writer(&mut writer, self)?;
        }
        writer.flush()?;
        Ok(())
    }
}
