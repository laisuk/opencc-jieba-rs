//! Dictionary-pack generation utilities.
//!
//! This module is available only when the `dictionary-build` feature is
//! enabled. It is intended for build tools that generate runtime dictionary
//! packs from the repository's plaintext `./dicts` source directory.
//!
//! The runtime dictionary representation, individual maps, metadata, and
//! logical slot storage remain private implementation details.
//!
//! # Source dictionaries
//!
//! The plaintext OpenCC dictionary sources are not included in the published
//! crate. Callers must provide the expected `./dicts` directory in the current
//! working directory.
//!
//! # Custom dictionaries
//!
//! Custom conversion dictionaries can be applied while generating a pack by
//! using [`write_json_pretty_with_custom_dict_files`] or
//! [`write_zstd_with_custom_dict_files`].
//!
//! Custom files are applied to an in-memory copy of the base dictionaries.
//! The source files under `./dicts` are never modified.
//!
//! [`CustomDictMode::Append`](crate::CustomDictMode::Append) preserves the base
//! slot and adds or replaces mappings from the supplied custom files.
//! [`CustomDictMode::Override`](crate::CustomDictMode::Override) clears the
//! target slot before applying the custom files.
//!
//! Custom dictionary loading is strict: malformed custom input causes
//! generation to fail rather than silently producing a partially customized
//! pack.

use crate::dictionary_lib::Dictionary;
use crate::CustomDictFileSpec;
use std::io;
use std::path::Path;

/// Generates a pretty-printed JSON dictionary pack from the base dictionaries.
///
/// Dictionary data is loaded from the `./dicts` directory and serialized with
/// deterministic dictionary-map key ordering.
///
/// # Errors
///
/// Returns an error if a source dictionary cannot be read or the generated
/// JSON cannot be written.
///
/// # Since
///
/// v0.8.0
pub fn write_json_pretty(path: impl AsRef<Path>) -> io::Result<()> {
    Dictionary::from_dicts().save_json(path, true)
}

/// Generates a Zstandard-compressed JSON dictionary pack from the base
/// dictionaries.
///
/// Dictionary data is loaded from the `./dicts` directory, serialized as
/// compact JSON with deterministic dictionary-map key ordering, and compressed
/// using Zstandard.
///
/// # Errors
///
/// Returns an error if a source dictionary cannot be read, serialization
/// fails, or the compressed output cannot be written.
///
/// # Since
///
/// v0.8.0
pub fn write_zstd(path: impl AsRef<Path>) -> io::Result<()> {
    Dictionary::from_dicts().save_json_compressed(path)
}

/// Generates a pretty-printed JSON dictionary pack with custom conversion
/// dictionary files applied.
///
/// The base dictionary is first loaded from `./dicts`. The supplied
/// [`CustomDictFileSpec`] values are then applied in order before the resulting
/// dictionary is serialized.
///
/// `Append` specs preserve existing mappings in the target slot while adding
/// or replacing mappings from the custom files. `Override` specs clear the
/// target slot before loading their custom files.
///
/// The operation is non-destructive: custom mappings are applied only to the
/// in-memory dictionary used to generate the output. The source files under
/// `./dicts` are never modified.
///
/// # Errors
///
/// Returns an error if a base or custom dictionary file cannot be read, a
/// custom dictionary contains malformed input, or the generated JSON cannot
/// be written.
///
/// # Since
///
/// v0.8.0
pub fn write_json_pretty_with_custom_dict_files<P>(
    path: impl AsRef<Path>,
    specs: &[CustomDictFileSpec<P>],
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    Dictionary::from_dicts_with_custom_files(specs)?.save_json(path, true)
}

/// Generates a Zstandard-compressed JSON dictionary pack with custom conversion
/// dictionary files applied.
///
/// The base dictionary is first loaded from `./dicts`. The supplied
/// [`CustomDictFileSpec`] values are then applied in order before the resulting
/// dictionary is serialized as compact JSON and compressed using Zstandard.
///
/// `Append` specs preserve existing mappings in the target slot while adding
/// or replacing mappings from the custom files. `Override` specs clear the
/// target slot before loading their custom files.
///
/// The operation is non-destructive: custom mappings are applied only to the
/// in-memory dictionary used to generate the output. The source files under
/// `./dicts` are never modified.
///
/// # Errors
///
/// Returns an error if a base or custom dictionary file cannot be read, a
/// custom dictionary contains malformed input, serialization fails, or the
/// compressed output cannot be written.
///
/// # Since
///
/// v0.8.0
pub fn write_zstd_with_custom_dict_files<P>(
    path: impl AsRef<Path>,
    specs: &[CustomDictFileSpec<P>],
) -> io::Result<()>
where
    P: AsRef<Path>,
{
    Dictionary::from_dicts_with_custom_files(specs)?.save_json_compressed(path)
}
