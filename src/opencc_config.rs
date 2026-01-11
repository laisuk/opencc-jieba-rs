/// OpenCC conversion configuration (strongly-typed).
///
/// This enum represents the supported conversion “modes” (e.g. Simplified → Traditional).
/// It is used by [`OpenCC::convert_with_config`] to avoid string parsing in hot paths.
///
/// # ABI / FFI
///
/// `OpenccConfig` is marked with `#[repr(u32)]`, so each variant has a stable numeric value.
/// This is suitable for C FFI where configs are passed as `uint32_t` (`opencc_config_t`).
///
/// When accepting configs from FFI, **do not** `transmute`; use [`OpenccConfig::from_ffi`]
/// to validate values.
///
/// # String parsing
///
/// For convenience and backwards compatibility, configs can also be parsed from strings
/// via `TryFrom<&str>` (case-insensitive), which powers [`OpenCC::convert`].
///
/// # Variants
///
/// | Variant | Name   | Description                               | Punctuation parameter used? |
/// |--------:|--------|-------------------------------------------|-----------------------------|
/// | 1       | `S2t`  | Simplified → Traditional                   | ✅ (passed through)         |
/// | 2       | `S2tw` | Simplified → Traditional (Taiwan)          | ✅                          |
/// | 3       | `S2twp`| Simplified → Taiwan (with phrases)         | ✅                          |
/// | 4       | `S2hk` | Simplified → Hong Kong                     | ✅                          |
/// | 5       | `T2s`  | Traditional → Simplified                   | ✅                          |
/// | 6       | `T2tw` | Traditional → Taiwan                       | ❌ (ignored)                |
/// | 7       | `T2twp`| Traditional → Taiwan (with phrases)        | ❌ (ignored)                |
/// | 8       | `T2hk` | Traditional → Hong Kong                    | ❌ (ignored)                |
/// | 9       | `Tw2s` | Taiwan → Simplified                        | ✅                          |
/// | 10      | `Tw2sp`| Taiwan → Simplified (variant)              | ✅                          |
/// | 11      | `Tw2t` | Taiwan → Traditional                       | ❌ (ignored)                |
/// | 12      | `Tw2tp`| Taiwan → Traditional (variant)             | ❌ (ignored)                |
/// | 13      | `Hk2s` | Hong Kong → Simplified                     | ✅                          |
/// | 14      | `Hk2t` | Hong Kong → Traditional                    | ❌ (ignored)                |
/// | 15      | `Jp2t` | Japanese (Kanji variants) → Traditional     | ❌ (ignored)                |
/// | 16      | `T2jp` | Traditional → Japanese (Kanji variants)     | ❌ (ignored)                |
/// # Since
///
/// Available since **v0.8.4**.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenccConfig {
    /// Simplified Chinese → Traditional Chinese.
    S2t = 1,

    /// Simplified Chinese → Traditional Chinese (Taiwan standard).
    S2tw = 2,

    /// Simplified Chinese → Traditional Chinese (Taiwan, with phrases).
    S2twp = 3,

    /// Simplified Chinese → Traditional Chinese (Hong Kong standard).
    S2hk = 4,

    /// Traditional Chinese → Simplified Chinese.
    T2s = 5,

    /// Traditional Chinese → Taiwanese variant.
    T2tw = 6,

    /// Traditional Chinese → Taiwanese variant (with phrases).
    T2twp = 7,

    /// Traditional Chinese → Hong Kong variant.
    T2hk = 8,

    /// Taiwanese variant → Simplified Chinese.
    Tw2s = 9,

    /// Taiwanese variant → Simplified Chinese (with phrases).
    Tw2sp = 10,

    /// Taiwanese variant → Traditional Chinese.
    Tw2t = 11,

    /// Taiwanese variant → Traditional Chinese (with phrases).
    Tw2tp = 12,

    /// Hong Kong variant → Simplified Chinese.
    Hk2s = 13,

    /// Hong Kong variant → Traditional Chinese.
    Hk2t = 14,

    /// Japanese Kanji → Traditional Chinese.
    Jp2t = 15,

    /// Traditional Chinese → Japanese Kanji.
    T2jp = 16,
}

impl TryFrom<&str> for OpenccConfig {
    /// Parses a configuration name (case-insensitive).
    ///
    /// Accepted names: `"s2t"`, `"s2tw"`, `"s2twp"`, `"s2hk"`, `"t2s"`, `"t2tw"`, `"t2twp"`,
    /// `"t2hk"`, `"tw2s"`, `"tw2sp"`, `"tw2t"`, `"tw2tp"`, `"hk2s"`, `"hk2t"`, `"jp2t"`, `"t2jp"`.
    ///
    /// This is primarily used by [`OpenCC::convert`] to support legacy `&str` configs.
    /// # Since
    ///
    /// Available since **v0.8.4**.
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_ascii_lowercase().as_str() {
            "s2t" => Ok(Self::S2t),
            "s2tw" => Ok(Self::S2tw),
            "s2twp" => Ok(Self::S2twp),
            "s2hk" => Ok(Self::S2hk),
            "t2s" => Ok(Self::T2s),
            "t2tw" => Ok(Self::T2tw),
            "t2twp" => Ok(Self::T2twp),
            "t2hk" => Ok(Self::T2hk),
            "tw2s" => Ok(Self::Tw2s),
            "tw2sp" => Ok(Self::Tw2sp),
            "tw2t" => Ok(Self::Tw2t),
            "tw2tp" => Ok(Self::Tw2tp),
            "hk2s" => Ok(Self::Hk2s),
            "hk2t" => Ok(Self::Hk2t),
            "jp2t" => Ok(Self::Jp2t),
            "t2jp" => Ok(Self::T2jp),
            _ => Err(()),
        }
    }
}

impl OpenccConfig {
    /// Converts an FFI numeric config value into [`OpenccConfig`].
    ///
    /// Returns `None` for unknown values. This is the **only** supported way to accept
    /// configuration numbers from C FFI (`uint32_t` / `opencc_config_t`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use opencc_jieba_rs::OpenccConfig;
    ///
    /// assert_eq!(OpenccConfig::from_ffi(1), Some(OpenccConfig::S2t));
    /// assert_eq!(OpenccConfig::from_ffi(999), None);
    /// ```
    /// # Since
    ///
    /// Available since **v0.8.4**.
    #[inline]
    pub fn from_ffi(v: u32) -> Option<Self> {
        Some(match v {
            1 => Self::S2t,
            2 => Self::S2tw,
            3 => Self::S2twp,
            4 => Self::S2hk,
            5 => Self::T2s,
            6 => Self::T2tw,
            7 => Self::T2twp,
            8 => Self::T2hk,
            9 => Self::Tw2s,
            10 => Self::Tw2sp,
            11 => Self::Tw2t,
            12 => Self::Tw2tp,
            13 => Self::Hk2s,
            14 => Self::Hk2t,
            15 => Self::Jp2t,
            16 => Self::T2jp,
            _ => return None,
        })
    }
}