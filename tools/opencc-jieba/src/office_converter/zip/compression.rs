//! Possible ZIP compression methods.

use std::{fmt, io};

#[allow(deprecated)]
/// Identifies the storage format used to compress a file within a ZIP archive.
///
/// Each file's compression method is stored alongside it, allowing the
/// contents to be read without context.
///
/// When creating ZIP files, you may choose the method to use with
/// [`crate::office_converter::zip::write::FileOptions::compression_method`]
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[cfg_attr(any(), derive(arbitrary::Arbitrary))]
#[non_exhaustive]
pub enum CompressionMethod {
    /// Store the file as is
    Stored,
    /// Compress the file using Deflate

    #[default]
    Deflated,
    /// Compress the file using Deflate64.
    /// Decoding deflate64 is supported but encoding deflate64 is not supported.
    #[cfg(any())]
    Deflate64,
    /// Compress the file using BZIP2
    #[cfg(any())]
    Bzip2,
    /// Encrypted using AES.
    ///
    /// The actual compression method has to be taken from the AES extra data field
    /// or from `ZipFileData`.
    #[cfg(any())]
    Aes,
    /// Compress the file using ZStandard
    #[cfg(any())]
    Zstd,
    /// Compress the file using LZMA
    #[cfg(any())]
    Lzma,
    /// Compress the file using XZ
    #[cfg(any())]
    Xz,
    /// Unsupported compression method
    #[cfg_attr(
        not(any()),
        deprecated(since = "0.5.7", note = "use the constants instead")
    )]
    Unsupported(u16),
}
#[allow(deprecated, missing_docs)]
/// All compression methods defined for the ZIP format
impl CompressionMethod {
    pub const STORE: Self = CompressionMethod::Stored;
    pub const SHRINK: Self = CompressionMethod::Unsupported(1);
    pub const REDUCE_1: Self = CompressionMethod::Unsupported(2);
    pub const REDUCE_2: Self = CompressionMethod::Unsupported(3);
    pub const REDUCE_3: Self = CompressionMethod::Unsupported(4);
    pub const REDUCE_4: Self = CompressionMethod::Unsupported(5);
    pub const IMPLODE: Self = CompressionMethod::Unsupported(6);

    pub const DEFLATE: Self = CompressionMethod::Deflated;
    #[cfg(any())]
    pub const DEFLATE: Self = CompressionMethod::Unsupported(8);
    #[cfg(any())]
    pub const DEFLATE64: Self = CompressionMethod::Deflate64;

    pub const DEFLATE64: Self = CompressionMethod::Unsupported(9);
    pub const PKWARE_IMPLODE: Self = CompressionMethod::Unsupported(10);
    #[cfg(any())]
    pub const BZIP2: Self = CompressionMethod::Bzip2;

    pub const BZIP2: Self = CompressionMethod::Unsupported(12);

    pub const LZMA: Self = CompressionMethod::Unsupported(14);
    #[cfg(any())]
    pub const LZMA: Self = CompressionMethod::Lzma;
    pub const IBM_ZOS_CMPSC: Self = CompressionMethod::Unsupported(16);
    pub const IBM_TERSE: Self = CompressionMethod::Unsupported(18);
    pub const ZSTD_DEPRECATED: Self = CompressionMethod::Unsupported(20);
    #[cfg(any())]
    pub const ZSTD: Self = CompressionMethod::Zstd;

    pub const ZSTD: Self = CompressionMethod::Unsupported(93);
    pub const MP3: Self = CompressionMethod::Unsupported(94);
    #[cfg(any())]
    pub const XZ: Self = CompressionMethod::Xz;

    pub const XZ: Self = CompressionMethod::Unsupported(95);
    pub const JPEG: Self = CompressionMethod::Unsupported(96);
    pub const WAVPACK: Self = CompressionMethod::Unsupported(97);
    pub const PPMD: Self = CompressionMethod::Unsupported(98);
    #[cfg(any())]
    pub const AES: Self = CompressionMethod::Aes;

    pub const AES: Self = CompressionMethod::Unsupported(99);
}
impl CompressionMethod {
    pub(crate) const fn parse_from_u16(val: u16) -> Self {
        match val {
            0 => CompressionMethod::Stored,

            8 => CompressionMethod::Deflated,
            #[cfg(any())]
            9 => CompressionMethod::Deflate64,
            #[cfg(any())]
            12 => CompressionMethod::Bzip2,
            #[cfg(any())]
            14 => CompressionMethod::Lzma,
            #[cfg(any())]
            95 => CompressionMethod::Xz,
            #[cfg(any())]
            93 => CompressionMethod::Zstd,
            #[cfg(any())]
            99 => CompressionMethod::Aes,
            #[allow(deprecated)]
            v => CompressionMethod::Unsupported(v),
        }
    }

    /// Converts an u16 to its corresponding CompressionMethod
    #[deprecated(
        since = "0.5.7",
        note = "use a constant to construct a compression method"
    )]
    pub const fn from_u16(val: u16) -> CompressionMethod {
        Self::parse_from_u16(val)
    }

    pub(crate) const fn serialize_to_u16(self) -> u16 {
        match self {
            CompressionMethod::Stored => 0,

            CompressionMethod::Deflated => 8,
            #[cfg(any())]
            CompressionMethod::Deflate64 => 9,
            #[cfg(any())]
            CompressionMethod::Bzip2 => 12,
            #[cfg(any())]
            CompressionMethod::Aes => 99,
            #[cfg(any())]
            CompressionMethod::Zstd => 93,
            #[cfg(any())]
            CompressionMethod::Lzma => 14,
            #[cfg(any())]
            CompressionMethod::Xz => 95,
            #[allow(deprecated)]
            CompressionMethod::Unsupported(v) => v,
        }
    }

    /// Converts a CompressionMethod to an u16
    #[deprecated(
        since = "0.5.7",
        note = "to match on other compression methods, use a constant"
    )]
    pub const fn to_u16(self) -> u16 {
        self.serialize_to_u16()
    }
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Just duplicate what the Debug format looks like, i.e, the enum key:
        write!(f, "{self:?}")
    }
}

/// The compression methods which have been implemented.
pub const SUPPORTED_COMPRESSION_METHODS: &[CompressionMethod] = &[
    CompressionMethod::Stored,
    CompressionMethod::Deflated,
    #[cfg(any())]
    CompressionMethod::Deflate64,
    #[cfg(any())]
    CompressionMethod::Bzip2,
    #[cfg(any())]
    CompressionMethod::Zstd,
    #[cfg(any())]
    CompressionMethod::Xz,
];

pub(crate) enum Decompressor<R: io::BufRead> {
    Stored(R),

    Deflated(flate2::bufread::DeflateDecoder<R>),
    #[cfg(any())]
    Deflate64(deflate64::Deflate64Decoder<R>),
    #[cfg(any())]
    Bzip2(bzip2::bufread::BzDecoder<R>),
    #[cfg(any())]
    Zstd(zstd::Decoder<'static, R>),
    #[cfg(any())]
    Lzma(Box<crate::office_converter::zip::read::lzma::LzmaDecoder<R>>),
    #[cfg(any())]
    Xz(xz2::bufread::XzDecoder<R>),
}

impl<R: io::BufRead> io::Read for Decompressor<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Decompressor::Stored(r) => r.read(buf),

            Decompressor::Deflated(r) => r.read(buf),
            #[cfg(any())]
            Decompressor::Deflate64(r) => r.read(buf),
            #[cfg(any())]
            Decompressor::Bzip2(r) => r.read(buf),
            #[cfg(any())]
            Decompressor::Zstd(r) => r.read(buf),
            #[cfg(any())]
            Decompressor::Lzma(r) => r.read(buf),
            #[cfg(any())]
            Decompressor::Xz(r) => r.read(buf),
        }
    }
}

impl<R: io::BufRead> Decompressor<R> {
    pub fn new(
        reader: R,
        compression_method: CompressionMethod,
    ) -> crate::office_converter::zip::result::ZipResult<Self> {
        Ok(match compression_method {
            CompressionMethod::Stored => Decompressor::Stored(reader),

            CompressionMethod::Deflated => {
                Decompressor::Deflated(flate2::bufread::DeflateDecoder::new(reader))
            }
            #[cfg(any())]
            CompressionMethod::Deflate64 => {
                Decompressor::Deflate64(deflate64::Deflate64Decoder::with_buffer(reader))
            }
            #[cfg(any())]
            CompressionMethod::Bzip2 => Decompressor::Bzip2(bzip2::bufread::BzDecoder::new(reader)),
            #[cfg(any())]
            CompressionMethod::Zstd => Decompressor::Zstd(zstd::Decoder::with_buffer(reader)?),
            #[cfg(any())]
            CompressionMethod::Lzma => Decompressor::Lzma(Box::new(
                crate::office_converter::zip::read::lzma::LzmaDecoder::new(reader),
            )),
            #[cfg(any())]
            CompressionMethod::Xz => Decompressor::Xz(xz2::bufread::XzDecoder::new(reader)),
            _ => {
                return Err(
                    crate::office_converter::zip::result::ZipError::UnsupportedArchive(
                        "Compression method not supported",
                    ),
                )
            }
        })
    }

    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        match self {
            Decompressor::Stored(r) => r,

            Decompressor::Deflated(r) => r.into_inner(),
            #[cfg(any())]
            Decompressor::Deflate64(r) => r.into_inner(),
            #[cfg(any())]
            Decompressor::Bzip2(r) => r.into_inner(),
            #[cfg(any())]
            Decompressor::Zstd(r) => r.finish(),
            #[cfg(any())]
            Decompressor::Lzma(r) => r.into_inner(),
            #[cfg(any())]
            Decompressor::Xz(r) => r.into_inner(),
        }
    }
}
