//! CJK Compatibility Ideograph normalization.
//!
//! This module normalizes Unicode CJK Compatibility Ideographs to their
//! UnicodeData decomposition targets. It is an optional Unicode compatibility
//! normalization pre-pass, not an OpenCC dictionary conversion.
//!
//! The built-in table is loaded from `data/CJK_Compatibility_Ideographs.txt`
//! with [`include_bytes!`], parsed once, and cached in dense runtime lookup
//! tables with [`std::sync::OnceLock`]. Unmapped compatibility ideographs remain unchanged,
//! as do characters outside the compatibility ideograph ranges.

use std::sync::OnceLock;

static COMPAT_DATA: &[u8] = include_bytes!("data/CJK_Compatibility_Ideographs.txt");

const BMP_START: u32 = 0xF900;
const BMP_END: u32 = 0xFAFF;
const BMP_LEN: usize = (BMP_END - BMP_START + 1) as usize;

const SUPP_START: u32 = 0x2F800;
const SUPP_END: u32 = 0x2FA1F;
const SUPP_LEN: usize = (SUPP_END - SUPP_START + 1) as usize;

static COMPAT_TABLE: OnceLock<CompatIdeographs> = OnceLock::new();

/// Dense lookup tables for CJK Compatibility Ideograph normalization.
///
/// The built-in table maps compatibility ideographs to their UnicodeData
/// decomposition targets. Each supported range is stored densely for fast
/// character lookup. Characters without a mapping are initialized to themselves,
/// so normalization preserves unmapped compatibility ideographs unchanged.
#[derive(Debug, Clone)]
pub(crate) struct CompatIdeographs {
    bmp: [char; BMP_LEN],
    supp: [char; SUPP_LEN],
}

impl Default for CompatIdeographs {
    fn default() -> Self {
        let mut bmp = ['\0'; BMP_LEN];
        let mut supp = ['\0'; SUPP_LEN];

        for (i, slot) in bmp.iter_mut().enumerate() {
            *slot = char::from_u32(BMP_START + i as u32).unwrap();
        }

        for (i, slot) in supp.iter_mut().enumerate() {
            *slot = char::from_u32(SUPP_START + i as u32).unwrap();
        }

        Self { bmp, supp }
    }
}

impl CompatIdeographs {
    /// Returns the cached built-in compatibility ideograph normalizer.
    ///
    /// The bundled mapping data is parsed at most once per process. Subsequent
    /// calls reuse the same dense lookup tables.
    pub(crate) fn builtin() -> &'static Self {
        COMPAT_TABLE.get_or_init(|| {
            let text = std::str::from_utf8(COMPAT_DATA)
                .expect("CJK_Compatibility_Ideographs.txt must be valid UTF-8");

            Self::from_text(text)
                .unwrap_or_else(|err| panic!("invalid CJK_Compatibility_Ideographs.txt: {err}"))
        })
    }

    /// Builds a compatibility ideograph normalizer from UTF-8 mapping text.
    ///
    /// This is mainly useful for tests or custom data. The expected format is
    /// one tab-separated `source<TAB>target` pair per line, with `#` comments
    /// and blank lines ignored.
    pub(crate) fn from_text(text: &str) -> Result<Self, String> {
        let mut table = Self::default();

        for (index, raw_line) in text.lines().enumerate() {
            let line_no = index + 1;

            if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
                continue;
            }

            let mut parts = raw_line.split('\t');

            let src_text = parts
                .next()
                .map(str::trim)
                .ok_or_else(|| format!("line {line_no}: missing source"))?;

            let dst_text = parts
                .next()
                .map(str::trim)
                .ok_or_else(|| format!("line {line_no}: missing target"))?;

            if parts.next().is_some() {
                return Err(format!("line {line_no}: too many columns"));
            }

            let src = single_char(src_text, line_no, "source")?;
            let dst = single_char(dst_text, line_no, "target")?;

            table.set(src, dst, line_no)?;
        }

        Ok(table)
    }

    fn set(&mut self, src: char, dst: char, line_no: usize) -> Result<(), String> {
        let u = src as u32;

        if (BMP_START..=BMP_END).contains(&u) {
            self.bmp[(u - BMP_START) as usize] = dst;
            return Ok(());
        }

        if (SUPP_START..=SUPP_END).contains(&u) {
            self.supp[(u - SUPP_START) as usize] = dst;
            return Ok(());
        }

        Err(format!(
            "line {line_no}: source U+{u:04X} is outside CJK Compatibility Ideograph ranges"
        ))
    }

    /// Normalizes one character if it has a compatibility mapping.
    ///
    /// Characters outside the CJK Compatibility Ideograph ranges, and
    /// compatibility ideographs without UnicodeData decomposition targets, are
    /// returned unchanged.
    ///
    #[inline(always)]
    pub(crate) fn normalize_char(&self, ch: char) -> char {
        let u = ch as u32;

        if (BMP_START..=BMP_END).contains(&u) {
            return self.bmp[(u - BMP_START) as usize];
        }

        if (SUPP_START..=SUPP_END).contains(&u) {
            return self.supp[(u - SUPP_START) as usize];
        }

        ch
    }

    /// Normalizes a mutable character slice in place.
    ///
    /// This is useful when text has already been collected into a reusable
    /// `Vec<char>` before segmentation.
    ///
    #[cfg(test)]
    pub(crate) fn normalize_in_place(&self, chars: &mut [char]) {
        for ch in chars {
            *ch = self.normalize_char(*ch);
        }
    }

    /// Normalizes all mapped CJK Compatibility Ideographs in `input`.
    ///
    /// This returns a new string and leaves ordinary Chinese text unchanged.
    ///
    pub(crate) fn normalize(&self, input: &str) -> String {
        crate::opencc::map_chars_cow(input, |ch| self.normalize_char(ch)).into_owned()
    }
}

fn single_char(text: &str, line_no: usize, field: &str) -> Result<char, String> {
    let mut chars = text.chars();

    let ch = chars
        .next()
        .ok_or_else(|| format!("line {line_no}: empty {field}"))?;

    if chars.next().is_some() {
        return Err(format!(
            "line {line_no}: {field} must be exactly one character"
        ));
    }

    Ok(ch)
}

/// Normalizes mapped CJK Compatibility Ideographs using the built-in table.
///
/// This is a convenience wrapper around [`CompatIdeographs::builtin`] and
/// [`CompatIdeographs::normalize`]. It performs Unicode compatibility
/// normalization as an optional pre-pass before OpenCC conversion.
///
pub(crate) fn normalize_compat_ideographs(input: &str) -> String {
    CompatIdeographs::builtin().normalize(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bmp_compat_ideographs() {
        let table = CompatIdeographs::builtin();

        assert_eq!(table.normalize("金庸"), "金庸");
        assert_eq!(table.normalize("龜龜"), "龜龜");
        assert_eq!(table.normalize("樂天"), "樂天");
    }

    #[test]
    fn leaves_normal_text_unchanged() {
        let table = CompatIdeographs::builtin();

        assert_eq!(table.normalize("金庸寫小說"), "金庸寫小說");
        assert_eq!(table.normalize("abc123，測試。"), "abc123，測試。");
    }

    #[test]
    fn normalizes_in_place() {
        let table = CompatIdeographs::builtin();

        let mut chars: Vec<char> = "金龜樂".chars().collect();
        table.normalize_in_place(&mut chars);

        assert_eq!(chars.iter().collect::<String>(), "金龜樂");
    }

    #[test]
    fn unmapped_compat_ideographs_stay_self() {
        let table = CompatIdeographs::builtin();

        // U+FA11 is documented as having no UnicodeData decomposition mapping.
        assert_eq!(table.normalize_char('﨑'), '﨑');
    }

    #[test]
    fn parses_custom_table() {
        let table = CompatIdeographs::from_text(
            "\
# comment
豈\t豈
金\t金
",
        )
        .unwrap();

        assert_eq!(table.normalize("豈金"), "豈金");
    }

    #[test]
    fn rejects_multi_char_source_or_target() {
        assert!(CompatIdeographs::from_text("豈x\t豈\n").is_err());
        assert!(CompatIdeographs::from_text("豈\t豈x\n").is_err());
    }

    #[test]
    fn rejects_source_outside_supported_ranges() {
        assert!(CompatIdeographs::from_text("金\t金\n").is_err());
    }
}
