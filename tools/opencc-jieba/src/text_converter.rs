//! # Shared text conversion pipeline
//!
//! Reusable text-to-text conversion support for the `opencc-jieba` CLI.
//!
//! This module separates transformation policy from command-specific I/O and
//! document-container handling. Consumers configure a [`TextConverter`] once,
//! then reuse it for plain text, Office/EPUB content, filename conversion, and
//! other text-bearing surfaces.
//!
//! The standard pipeline applies transformations in this order:
//!
//! 1. Optional compatibility normalization.
//! 2. OpenCC conversion, including optional punctuation conversion.
//! 3. Optional DeTofu fallback replacement.
//!
//! Jieba user dictionaries and custom OpenCC dictionaries are loaded onto the
//! [`OpenCC`] instance before the converter is created, so the returned
//! converter automatically uses that configured instance.

use std::borrow::Cow;

use opencc_jieba_rs::{DetofuLevel, OpenCC};

/// Compatibility normalization performed before OpenCC conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NormalizationMode {
    /// Do not normalize compatibility forms.
    #[default]
    None,

    /// Normalize CJK Compatibility Ideographs.
    Compat,

    /// Normalize both CJK Compatibility Ideographs and extended curated forms.
    CompatExtended,
}

/// Configuration captured by the standard text-conversion pipeline.
#[derive(Clone, Copy, Debug)]
pub struct TextConverterOptions<'a> {
    /// OpenCC conversion configuration, for example `"s2t"` or `"t2s"`.
    pub config: &'a str,

    /// Whether punctuation conversion is enabled.
    pub punctuation: bool,

    /// Compatibility normalization mode.
    pub normalization: NormalizationMode,

    /// Whether DeTofu fallback replacement is applied after conversion.
    ///
    /// The CLI currently uses [`DetofuLevel::ExtB`], which covers Extension B
    /// and all supported later extension mappings.
    pub detofu: bool,
}

/// Generic reusable text-to-text transformation.
///
/// The converter deliberately knows nothing about files, encodings, Office
/// packages, EPUB archives, or command-line arguments. It simply wraps an
/// `Fn(&str) -> String`.
pub struct TextConverter<F> {
    convert: F,
}

impl<F> TextConverter<F>
where
    F: Fn(&str) -> String,
{
    /// Creates a converter from a closure or function.
    #[inline]
    pub fn new(convert: F) -> Self {
        Self { convert }
    }

    /// Converts one text fragment.
    #[inline]
    pub fn convert(&self, text: &str) -> String {
        (self.convert)(text)
    }
}

/// Builds the standard `opencc-jieba` conversion pipeline.
///
/// The supplied [`OpenCC`] instance may already contain Jieba user dictionaries
/// and custom OpenCC conversion dictionaries. Those customizations are therefore
/// automatically honored by every consumer of the returned converter.
pub fn create_text_converter<'a>(
    opencc: &'a OpenCC,
    options: TextConverterOptions<'a>,
) -> TextConverter<impl Fn(&str) -> String + 'a> {
    TextConverter::new(move |input| {
        let normalized = match options.normalization {
            NormalizationMode::None => Cow::Borrowed(input),
            NormalizationMode::Compat => Cow::Owned(opencc.normalize_compat(input)),
            NormalizationMode::CompatExtended => {
                Cow::Owned(opencc.normalize_compat_extended(input))
            }
        };

        let converted =
            opencc.convert(normalized.as_ref(), options.config, options.punctuation);

        if options.detofu {
            opencc.detofu(&converted, DetofuLevel::ExtB)
        } else {
            converted
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_converter_wraps_plain_closure() {
        let converter = TextConverter::new(|text: &str| text.replace("汉语", "漢語"));

        assert_eq!(converter.convert("汉语"), "漢語");
    }

    #[test]
    fn pipeline_applies_extended_normalize_convert_and_detofu() {
        let opencc = OpenCC::new();

        let converter = create_text_converter(
            &opencc,
            TextConverterOptions {
                config: "t2s",
                punctuation: false,
                normalization: NormalizationMode::CompatExtended,
                detofu: true,
            },
        );

        assert_eq!(converter.convert("聼𧜗"), "听䘞");
    }
}
