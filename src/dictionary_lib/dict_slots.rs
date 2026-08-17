//! Dictionary-slot and custom conversion dictionary specifications.
//!
//! This module defines the public slot identifiers and custom-dictionary
//! specifications used by `opencc-jieba-rs`.
//!
//! Custom conversion dictionaries are intentionally **post-load**:
//!
//! - [`OpenCC`](crate::OpenCC) first loads its built-in or external conversion
//!   [`Dictionary`](crate::dictionary_lib::Dictionary).
//! - Custom entries are then applied to one or more logical [`DictSlot`] values.
//! - Jieba user dictionaries remain a separate concern and continue to be loaded
//!   through [`OpenCC::load_user_dict`](crate::OpenCC::load_user_dict).
//!
//! This keeps conversion policy and segmentation policy independent.

use std::path::Path;

/// Identifies a logical conversion dictionary slot.
///
/// Each slot corresponds to one [`DictMap`](crate::dictionary_lib::DictMap)
/// stored inside [`Dictionary`](crate::dictionary_lib::Dictionary).
///
/// Custom dictionaries can append entries to a slot or replace that slot's
/// contents entirely.
///
/// # Important
///
/// These slots affect **OpenCC conversion mappings only**. They do not modify
/// Jieba segmentation. Domain-specific phrases that must remain a single Jieba
/// token should also be added separately through a Jieba user dictionary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DictSlot {
    /// Simplified → Traditional character mappings.
    STCharacters,
    /// Simplified → Traditional phrase mappings.
    STPhrases,
    /// Traditional → Simplified character mappings.
    TSCharacters,
    /// Traditional → Simplified phrase mappings.
    TSPhrases,
    /// Traditional → Taiwan phrase mappings.
    TWPhrases,
    /// Taiwan → Traditional reverse phrase mappings.
    TWPhrasesRev,
    /// Traditional → Hong Kong phrase mappings.
    HKPhrases,
    /// Hong Kong → Traditional reverse phrase mappings.
    HKPhrasesRev,
    /// Traditional → Taiwan regional variant mappings.
    TWVariants,
    /// Traditional → Taiwan regional phrase-variant mappings.
    TWVariantsPhrases,
    /// Taiwan → Traditional reverse variant mappings.
    TWVariantsRev,
    /// Taiwan → Traditional reverse phrase-variant mappings.
    TWVariantsRevPhrases,
    /// Traditional → Hong Kong regional variant mappings.
    HKVariants,
    /// Traditional → Hong Kong regional phrase-variant mappings.
    HKVariantsPhrases,
    /// Hong Kong → Traditional reverse variant mappings.
    HKVariantsRev,
    /// Hong Kong → Traditional reverse phrase-variant mappings.
    HKVariantsRevPhrases,
    /// Japanese Shinjitai → Traditional/Kyūjitai character mappings.
    JPSCharacters,
    /// Traditional/Kyūjitai → Japanese Shinjitai character mappings.
    JPSCharactersRev,
    /// Japanese Shinjitai → Traditional/Kyūjitai phrase mappings.
    JPSPhrases,
}

impl TryFrom<&str> for DictSlot {
    type Error = ();

    /// Parses a canonical dictionary slot name.
    ///
    /// Parsing is case-sensitive and does not trim whitespace.
    ///
    /// The historical physical filename stems
    /// `JPShinjitaiCharacters`, `JPShinjitaiCharactersRev`, and
    /// `JPShinjitaiPhrases` are also accepted as compatibility aliases.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "STCharacters" => Ok(Self::STCharacters),
            "STPhrases" => Ok(Self::STPhrases),

            "TSCharacters" => Ok(Self::TSCharacters),
            "TSPhrases" => Ok(Self::TSPhrases),

            "TWPhrases" => Ok(Self::TWPhrases),
            "TWPhrasesRev" => Ok(Self::TWPhrasesRev),

            "HKPhrases" => Ok(Self::HKPhrases),
            "HKPhrasesRev" => Ok(Self::HKPhrasesRev),

            "TWVariants" => Ok(Self::TWVariants),
            "TWVariantsPhrases" => Ok(Self::TWVariantsPhrases),
            "TWVariantsRev" => Ok(Self::TWVariantsRev),
            "TWVariantsRevPhrases" => Ok(Self::TWVariantsRevPhrases),

            "HKVariants" => Ok(Self::HKVariants),
            "HKVariantsPhrases" => Ok(Self::HKVariantsPhrases),
            "HKVariantsRev" => Ok(Self::HKVariantsRev),
            "HKVariantsRevPhrases" => Ok(Self::HKVariantsRevPhrases),

            "JPSCharacters" => Ok(Self::JPSCharacters),
            "JPSCharactersRev" => Ok(Self::JPSCharactersRev),
            "JPSPhrases" => Ok(Self::JPSPhrases),

            // Compatibility aliases matching physical dictionary filenames.
            "JPShinjitaiCharacters" => Ok(Self::JPSCharacters),
            "JPShinjitaiCharactersRev" => Ok(Self::JPSCharactersRev),
            "JPShinjitaiPhrases" => Ok(Self::JPSPhrases),

            _ => Err(()),
        }
    }
}

impl DictSlot {
    /// Every supported logical conversion dictionary slot.
    ///
    /// The order follows the field order used by the runtime [`Dictionary`].
    pub const ALL: &'static [Self] = &[
        Self::STCharacters,
        Self::STPhrases,
        Self::TSCharacters,
        Self::TSPhrases,
        Self::TWPhrases,
        Self::TWPhrasesRev,
        Self::HKPhrases,
        Self::HKPhrasesRev,
        Self::TWVariants,
        Self::TWVariantsPhrases,
        Self::TWVariantsRev,
        Self::TWVariantsRevPhrases,
        Self::HKVariants,
        Self::HKVariantsPhrases,
        Self::HKVariantsRev,
        Self::HKVariantsRevPhrases,
        Self::JPSCharacters,
        Self::JPSCharactersRev,
        Self::JPSPhrases,
    ];

    /// Returns the canonical public name of this slot.
    #[must_use]
    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::STCharacters => "STCharacters",
            Self::STPhrases => "STPhrases",
            Self::TSCharacters => "TSCharacters",
            Self::TSPhrases => "TSPhrases",
            Self::TWPhrases => "TWPhrases",
            Self::TWPhrasesRev => "TWPhrasesRev",
            Self::HKPhrases => "HKPhrases",
            Self::HKPhrasesRev => "HKPhrasesRev",
            Self::TWVariants => "TWVariants",
            Self::TWVariantsPhrases => "TWVariantsPhrases",
            Self::TWVariantsRev => "TWVariantsRev",
            Self::TWVariantsRevPhrases => "TWVariantsRevPhrases",
            Self::HKVariants => "HKVariants",
            Self::HKVariantsPhrases => "HKVariantsPhrases",
            Self::HKVariantsRev => "HKVariantsRev",
            Self::HKVariantsRevPhrases => "HKVariantsRevPhrases",
            Self::JPSCharacters => "JPSCharacters",
            Self::JPSCharactersRev => "JPSCharactersRev",
            Self::JPSPhrases => "JPSPhrases",
        }
    }

    /// Parses a canonical slot name without regard to ASCII case.
    ///
    /// Leading and trailing whitespace is ignored. The historical
    /// `JPShinjitai*` filename stems are also accepted as compatibility aliases.
    #[must_use]
    pub fn from_name_ignore_ascii_case(value: &str) -> Option<Self> {
        let value = value.trim();

        Self::ALL
            .iter()
            .copied()
            .find(|slot| slot.canonical_name().eq_ignore_ascii_case(value))
            .or_else(|| {
                if "JPShinjitaiCharacters".eq_ignore_ascii_case(value) {
                    Some(Self::JPSCharacters)
                } else if "JPShinjitaiCharactersRev".eq_ignore_ascii_case(value) {
                    Some(Self::JPSCharactersRev)
                } else if "JPShinjitaiPhrases".eq_ignore_ascii_case(value) {
                    Some(Self::JPSPhrases)
                } else {
                    None
                }
            })
    }
}

/// Controls how a custom conversion dictionary is applied to a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CustomDictMode {
    /// Merge custom entries into the existing slot.
    ///
    /// Conflicting keys use last-wins semantics.
    Append,

    /// Clear the target slot first, then insert the custom entries.
    Override,
}

/// Pair-based custom conversion dictionary specification.
///
/// This is the core no-I/O representation used by the post-load custom
/// dictionary path.
///
/// Custom conversion dictionaries do not alter Jieba segmentation.
#[derive(Debug, Clone)]
pub struct CustomDictSpec {
    /// Target conversion dictionary slot.
    pub slot: DictSlot,

    /// `(source, target)` conversion pairs.
    pub pairs: Vec<(String, String)>,

    /// Append or replace behavior for the target slot.
    pub mode: CustomDictMode,
}

/// File-based custom conversion dictionary specification.
///
/// Each file uses the OpenCC-style format:
///
/// ```text
/// source<TAB>target
/// ```
///
/// Multiple files are read and applied in the supplied order. The runtime
/// implementation should parse the files into pairs and reuse the same
/// post-load application path as [`CustomDictSpec`].
#[derive(Debug, Clone)]
pub struct CustomDictFileSpec<P = std::path::PathBuf>
where
    P: AsRef<Path>,
{
    /// Target conversion dictionary slot.
    pub slot: DictSlot,

    /// Custom OpenCC-style dictionary files.
    pub files: Vec<P>,

    /// Append or replace behavior for the target slot.
    pub mode: CustomDictMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_slots_round_trip_through_canonical_names() {
        for &slot in DictSlot::ALL {
            assert_eq!(DictSlot::try_from(slot.canonical_name()), Ok(slot));
        }
    }

    #[test]
    fn case_insensitive_parser_trims_whitespace() {
        assert_eq!(
            DictSlot::from_name_ignore_ascii_case("  stphrases  "),
            Some(DictSlot::STPhrases)
        );
        assert_eq!(
            DictSlot::from_name_ignore_ascii_case("JPSCHARACTERSREV"),
            Some(DictSlot::JPSCharactersRev)
        );
    }

    #[test]
    fn japanese_filename_aliases_are_supported() {
        assert_eq!(
            DictSlot::try_from("JPShinjitaiCharacters"),
            Ok(DictSlot::JPSCharacters)
        );
        assert_eq!(
            DictSlot::from_name_ignore_ascii_case("jpshinjitaiphrases"),
            Some(DictSlot::JPSPhrases)
        );
    }

    #[test]
    fn unknown_slots_are_rejected() {
        assert!(DictSlot::try_from("STPunctuations").is_err());
        assert!(DictSlot::try_from("STPhrases.txt").is_err());
        assert_eq!(DictSlot::from_name_ignore_ascii_case("unknown"), None);
    }
}
