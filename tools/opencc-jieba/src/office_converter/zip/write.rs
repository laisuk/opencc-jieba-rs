//! Types for creating ZIP archives

#[cfg(any())]
use crate::office_converter::zip::aes::AesWriter;
use crate::office_converter::zip::compression::CompressionMethod;
use crate::office_converter::zip::read::{parse_single_extra_field, Config, ZipArchive, ZipFile};
use crate::office_converter::zip::result::{invalid, ZipError, ZipResult};
use crate::office_converter::zip::spec::{self, FixedSizeBlock, Zip32CDEBlock};
#[cfg(any())]
use crate::office_converter::zip::types::AesMode;
use crate::office_converter::zip::types::{
    ffi, AesVendorVersion, DateTime, Zip64ExtraFieldBlock, ZipFileData, ZipLocalEntryBlock,
    ZipRawValues, MIN_VERSION,
};
use crate::office_converter::zip::write::ffi::S_IFLNK;
#[cfg(any())]
use core::num::NonZeroU64;
use crc32fast::Hasher;
use indexmap::IndexMap;
use std::borrow::ToOwned;
use std::default::Default;
use std::fmt::{Debug, Formatter};
use std::io;
use std::io::prelude::*;
use std::io::Cursor;
use std::io::{BufReader, SeekFrom};
use std::marker::PhantomData;
use std::mem;
use std::str::{from_utf8, Utf8Error};
use std::sync::Arc;

use flate2::{write::DeflateEncoder, Compression};

#[cfg(any())]
use bzip2::write::BzEncoder;

#[cfg(any())]
use zopfli::Options;

#[cfg(any())]
use std::io::BufWriter;
use std::mem::size_of;
use std::path::Path;

#[cfg(any())]
use zstd::stream::write::Encoder as ZstdEncoder;

enum MaybeEncrypted<W> {
    Unencrypted(W),
    #[cfg(any())]
    Aes(AesWriter<W>),
    ZipCrypto(crate::office_converter::zip::zipcrypto::ZipCryptoWriter<W>),
}

impl<W> Debug for MaybeEncrypted<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Don't print W, since it may be a huge Vec<u8>
        f.write_str(match self {
            MaybeEncrypted::Unencrypted(_) => "Unencrypted",
            #[cfg(any())]
            MaybeEncrypted::Aes(_) => "AES",
            MaybeEncrypted::ZipCrypto(_) => "ZipCrypto",
        })
    }
}

impl<W: Write> Write for MaybeEncrypted<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            MaybeEncrypted::Unencrypted(w) => w.write(buf),
            #[cfg(any())]
            MaybeEncrypted::Aes(w) => w.write(buf),
            MaybeEncrypted::ZipCrypto(w) => w.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            MaybeEncrypted::Unencrypted(w) => w.flush(),
            #[cfg(any())]
            MaybeEncrypted::Aes(w) => w.flush(),
            MaybeEncrypted::ZipCrypto(w) => w.flush(),
        }
    }
}

enum GenericZipWriter<W: Write + Seek> {
    Closed,
    Storer(MaybeEncrypted<W>),

    Deflater(DeflateEncoder<MaybeEncrypted<W>>),
    #[cfg(any())]
    ZopfliDeflater(zopfli::DeflateEncoder<MaybeEncrypted<W>>),
    #[cfg(any())]
    BufferedZopfliDeflater(BufWriter<zopfli::DeflateEncoder<MaybeEncrypted<W>>>),
    #[cfg(any())]
    Bzip2(BzEncoder<MaybeEncrypted<W>>),
    #[cfg(any())]
    Zstd(ZstdEncoder<'static, MaybeEncrypted<W>>),
    #[cfg(any())]
    Xz(xz2::write::XzEncoder<MaybeEncrypted<W>>),
}

impl<W: Write + Seek> Debug for GenericZipWriter<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Closed => f.write_str("Closed"),
            Storer(w) => f.write_fmt(format_args!("Storer({w:?})")),

            GenericZipWriter::Deflater(w) => {
                f.write_fmt(format_args!("Deflater({:?})", w.get_ref()))
            }
            #[cfg(any())]
            GenericZipWriter::ZopfliDeflater(_) => f.write_str("ZopfliDeflater"),
            #[cfg(any())]
            GenericZipWriter::BufferedZopfliDeflater(_) => f.write_str("BufferedZopfliDeflater"),
            #[cfg(any())]
            GenericZipWriter::Bzip2(w) => f.write_fmt(format_args!("Bzip2({:?})", w.get_ref())),
            #[cfg(any())]
            GenericZipWriter::Zstd(w) => f.write_fmt(format_args!("Zstd({:?})", w.get_ref())),
            #[cfg(any())]
            GenericZipWriter::Xz(w) => f.write_fmt(format_args!("Xz({:?})", w.get_ref())),
        }
    }
}

// Put the struct declaration in a private module to convince rustdoc to display ZipWriter nicely
pub(crate) mod zip_writer {
    use super::*;
    /// ZIP archive generator
    ///
    /// Handles the bookkeeping involved in building an archive, and provides an
    /// API to edit its contents.
    ///
    /// ```
    /// # fn doit() -> zip::result::ZipResult<()>
    /// # {
    /// # use zip::ZipWriter;
    /// use std::io::Write;
    /// use zip::write::SimpleFileOptions;
    ///
    /// // We use a buffer here, though you'd normally use a `File`
    /// let mut buf = [0; 65536];
    /// let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf[..]));
    ///
    /// let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    /// zip.start_file("hello_world.txt", options)?;
    /// zip.write(b"Hello, World!")?;
    ///
    /// // Apply the changes you've made.
    /// // Dropping the `ZipWriter` will have the same effect, but may silently fail
    /// zip.finish()?;
    ///
    /// # Ok(())
    /// # }
    /// # doit().unwrap();
    /// ```
    pub struct ZipWriter<W: Write + Seek> {
        pub(super) inner: GenericZipWriter<W>,
        pub(super) files: IndexMap<Box<str>, ZipFileData>,
        pub(super) stats: ZipWriterStats,
        pub(super) writing_to_file: bool,
        pub(super) writing_raw: bool,
        pub(super) comment: Box<[u8]>,
        pub(super) zip64_comment: Option<Box<[u8]>>,
        pub(super) flush_on_finish_file: bool,
    }

    impl<W: Write + Seek> Debug for ZipWriter<W> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!(
                "ZipWriter {{files: {:?}, stats: {:?}, writing_to_file: {}, writing_raw: {}, comment: {:?}, flush_on_finish_file: {}}}",
                self.files, self.stats, self.writing_to_file, self.writing_raw,
                self.comment, self.flush_on_finish_file))
        }
    }
}
#[doc(inline)]
pub use self::sealed::FileOptionExtension;
use crate::office_converter::zip::result::ZipError::UnsupportedArchive;
use crate::office_converter::zip::unstable::path_to_string;
use crate::office_converter::zip::unstable::LittleEndianWriteExt;
use crate::office_converter::zip::write::GenericZipWriter::{Closed, Storer};
use crate::office_converter::zip::zipcrypto::ZipCryptoKeys;
use crate::office_converter::zip::CompressionMethod::Stored;
pub use zip_writer::ZipWriter;

#[derive(Default, Debug)]
struct ZipWriterStats {
    hasher: Hasher,
    start: u64,
    bytes_written: u64,
}

mod sealed {
    use std::sync::Arc;

    use super::ExtendedFileOptions;

    pub trait Sealed {}
    /// File options Extensions
    #[doc(hidden)]
    pub trait FileOptionExtension: Default + Sealed {
        /// Extra Data
        fn extra_data(&self) -> Option<&Arc<Vec<u8>>>;
        /// Central Extra Data
        fn central_extra_data(&self) -> Option<&Arc<Vec<u8>>>;
    }
    impl Sealed for () {}
    impl FileOptionExtension for () {
        fn extra_data(&self) -> Option<&Arc<Vec<u8>>> {
            None
        }
        fn central_extra_data(&self) -> Option<&Arc<Vec<u8>>> {
            None
        }
    }
    impl Sealed for ExtendedFileOptions {}

    impl FileOptionExtension for ExtendedFileOptions {
        fn extra_data(&self) -> Option<&Arc<Vec<u8>>> {
            Some(&self.extra_data)
        }
        fn central_extra_data(&self) -> Option<&Arc<Vec<u8>>> {
            Some(&self.central_extra_data)
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum EncryptWith<'k> {
    #[cfg(any())]
    Aes {
        mode: AesMode,
        password: &'k str,
    },
    ZipCrypto(ZipCryptoKeys, PhantomData<&'k ()>),
}

#[cfg(any())]
impl<'a> arbitrary::Arbitrary<'a> for EncryptWith<'a> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        #[cfg(any())]
        if bool::arbitrary(u)? {
            return Ok(EncryptWith::Aes {
                mode: AesMode::arbitrary(u)?,
                password: u.arbitrary::<&str>()?,
            });
        }

        Ok(EncryptWith::ZipCrypto(
            ZipCryptoKeys::arbitrary(u)?,
            PhantomData,
        ))
    }
}

/// Metadata for a file to be written
#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub struct FileOptions<'k, T: FileOptionExtension> {
    pub(crate) compression_method: CompressionMethod,
    pub(crate) compression_level: Option<i64>,
    pub(crate) last_modified_time: DateTime,
    pub(crate) permissions: Option<u32>,
    pub(crate) large_file: bool,
    pub(crate) encrypt_with: Option<EncryptWith<'k>>,
    pub(crate) extended_options: T,
    pub(crate) alignment: u16,
    #[cfg(any())]
    pub(super) zopfli_buffer_size: Option<usize>,
}
/// Simple File Options. Can be copied and good for simple writing zip files
pub type SimpleFileOptions = FileOptions<'static, ()>;
/// Adds Extra Data and Central Extra Data. It does not implement copy.
pub type FullFileOptions<'k> = FileOptions<'k, ExtendedFileOptions>;
/// The Extension for Extra Data and Central Extra Data
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ExtendedFileOptions {
    extra_data: Arc<Vec<u8>>,
    central_extra_data: Arc<Vec<u8>>,
}

impl ExtendedFileOptions {
    /// Adds an extra data field, unless we detect that it's invalid.
    pub fn add_extra_data(
        &mut self,
        header_id: u16,
        data: Box<[u8]>,
        central_only: bool,
    ) -> ZipResult<()> {
        let len = data.len() + 4;
        if self.extra_data.len() + self.central_extra_data.len() + len > u16::MAX as usize {
            Err(invalid!("Extra data field would be longer than allowed"))
        } else {
            let field = if central_only {
                &mut self.central_extra_data
            } else {
                &mut self.extra_data
            };
            let vec = Arc::get_mut(field);
            let vec = match vec {
                Some(exclusive) => exclusive,
                None => {
                    *field = Arc::new(field.to_vec());
                    Arc::get_mut(field).unwrap()
                }
            };
            Self::add_extra_data_unchecked(vec, header_id, data)?;
            Self::validate_extra_data(vec, true)?;
            Ok(())
        }
    }

    pub(crate) fn add_extra_data_unchecked(
        vec: &mut Vec<u8>,
        header_id: u16,
        data: Box<[u8]>,
    ) -> Result<(), ZipError> {
        vec.reserve_exact(data.len() + 4);
        vec.write_u16_le(header_id)?;
        vec.write_u16_le(data.len() as u16)?;
        vec.write_all(&data)?;
        Ok(())
    }

    fn validate_extra_data(data: &[u8], disallow_zip64: bool) -> ZipResult<()> {
        let len = data.len() as u64;
        if len == 0 {
            return Ok(());
        }
        if len > u16::MAX as u64 {
            return Err(ZipError::Io(io::Error::other(
                "Extra-data field can't exceed u16::MAX bytes",
            )));
        }
        let mut data = Cursor::new(data);
        let mut pos = data.position();
        while pos < len {
            if len - data.position() < 4 {
                return Err(ZipError::Io(io::Error::other(
                    "Extra-data field doesn't have room for ID and length",
                )));
            }

            {
                use crate::office_converter::zip::unstable::LittleEndianReadExt;
                let header_id = data.read_u16_le()?;
                if EXTRA_FIELD_MAPPING.contains(&header_id) {
                    return Err(ZipError::Io(io::Error::other(
                        format!(
                            "Extra data header ID {header_id:#06} requires crate feature \"unreserved\"",
                        ),
                    )));
                }
                data.seek(SeekFrom::Current(-2))?;
            }
            parse_single_extra_field(&mut ZipFileData::default(), &mut data, pos, disallow_zip64)?;
            pos = data.position();
        }
        Ok(())
    }
}

impl Debug for ExtendedFileOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("ExtendedFileOptions {{extra_data: vec!{:?}.into(), central_extra_data: vec!{:?}.into()}}",
        self.extra_data, self.central_extra_data))
    }
}

#[cfg(any())]
impl<'a> arbitrary::Arbitrary<'a> for FileOptions<'a, ExtendedFileOptions> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut options = FullFileOptions {
            compression_method: CompressionMethod::arbitrary(u)?,
            compression_level: if bool::arbitrary(u)? {
                Some(u.int_in_range(0..=24)?)
            } else {
                None
            },
            last_modified_time: DateTime::arbitrary(u)?,
            permissions: Option::<u32>::arbitrary(u)?,
            large_file: bool::arbitrary(u)?,
            encrypt_with: Option::<EncryptWith>::arbitrary(u)?,
            alignment: u16::arbitrary(u)?,
            #[cfg(any())]
            zopfli_buffer_size: None,
            ..Default::default()
        };
        #[cfg(any())]
        if options.compression_method == CompressionMethod::Deflated && bool::arbitrary(u)? {
            options.zopfli_buffer_size =
                Some(if bool::arbitrary(u)? { 2 } else { 3 } << u.int_in_range(8..=20)?);
        }
        u.arbitrary_loop(Some(0), Some(10), |u| {
            options
                .add_extra_data(
                    u.int_in_range(2..=u16::MAX)?,
                    Box::<[u8]>::arbitrary(u)?,
                    bool::arbitrary(u)?,
                )
                .map_err(|_| arbitrary::Error::IncorrectFormat)?;
            Ok(core::ops::ControlFlow::Continue(()))
        })?;
        ZipWriter::new(Cursor::new(Vec::new()))
            .start_file("", options.clone())
            .map_err(|_| arbitrary::Error::IncorrectFormat)?;
        Ok(options)
    }
}

impl<T: FileOptionExtension> FileOptions<'_, T> {
    pub(crate) fn normalize(&mut self) {
        if !self.last_modified_time.is_valid() {
            self.last_modified_time = FileOptions::<T>::default().last_modified_time;
        }

        *self.permissions.get_or_insert(0o644) |= ffi::S_IFREG;
    }

    /// Set the compression method for the new file
    ///
    /// The default is `CompressionMethod::Deflated` if it is enabled. If not,
    /// `CompressionMethod::Bzip2` is the default if it is enabled. If neither `bzip2` nor `deflate`
    /// is enabled, `CompressionMethod::Zlib` is the default. If all else fails,
    /// `CompressionMethod::Stored` becomes the default and files are written uncompressed.
    #[must_use]
    pub const fn compression_method(mut self, method: CompressionMethod) -> Self {
        self.compression_method = method;
        self
    }

    /// Set the compression level for the new file
    ///
    /// `None` value specifies default compression level.
    ///
    /// Range of values depends on compression method:
    /// * `Deflated`: 10 - 264 for Zopfli, 0 - 9 for other encoders. Default is 24 if Zopfli is the
    ///   only encoder, or 6 otherwise.
    /// * `Bzip2`: 0 - 9. Default is 6
    /// * `Zstd`: -7 - 22, with zero being mapped to default level. Default is 3
    /// * others: only `None` is allowed
    #[must_use]
    pub const fn compression_level(mut self, level: Option<i64>) -> Self {
        self.compression_level = level;
        self
    }

    /// Set the last modified time
    ///
    /// The default is the current timestamp if the 'time' feature is enabled, and 1980-01-01
    /// otherwise
    #[must_use]
    pub const fn last_modified_time(mut self, mod_time: DateTime) -> Self {
        self.last_modified_time = mod_time;
        self
    }

    /// Set the permissions for the new file.
    ///
    /// The format is represented with unix-style permissions.
    /// The default is `0o644`, which represents `rw-r--r--` for files,
    /// and `0o755`, which represents `rwxr-xr-x` for directories.
    ///
    /// This method only preserves the file permissions bits (via a `& 0o777`) and discards
    /// higher file mode bits. So it cannot be used to denote an entry as a directory,
    /// symlink, or other special file type.
    #[must_use]
    pub const fn unix_permissions(mut self, mode: u32) -> Self {
        self.permissions = Some(mode & 0o777);
        self
    }

    /// Set whether the new file's compressed and uncompressed size is less than 4 GiB.
    ///
    /// If set to `false` and the file exceeds the limit, an I/O error is thrown and the file is
    /// aborted. If set to `true`, readers will require ZIP64 support and if the file does not
    /// exceed the limit, 20 B are wasted. The default is `false`.
    #[must_use]
    pub const fn large_file(mut self, large: bool) -> Self {
        self.large_file = large;
        self
    }

    pub(crate) fn with_deprecated_encryption(self, password: &[u8]) -> FileOptions<'static, T> {
        FileOptions {
            encrypt_with: Some(EncryptWith::ZipCrypto(
                ZipCryptoKeys::derive(password),
                PhantomData,
            )),
            ..self
        }
    }

    /// Set the AES encryption parameters.
    #[cfg(any())]
    pub fn with_aes_encryption(self, mode: AesMode, password: &str) -> FileOptions<'_, T> {
        FileOptions {
            encrypt_with: Some(EncryptWith::Aes { mode, password }),
            ..self
        }
    }

    /// Sets the size of the buffer used to hold the next block that Zopfli will compress. The
    /// larger, the buffer, the more effective the compression, but the more memory is required.
    /// A value of `None` indicates no buffer, which is recommended only when all non-empty writes
    /// are larger than about 32 KiB.
    #[must_use]
    #[cfg(any())]
    pub const fn with_zopfli_buffer(mut self, size: Option<usize>) -> Self {
        self.zopfli_buffer_size = size;
        self
    }

    /// Returns the compression level currently set.
    pub const fn get_compression_level(&self) -> Option<i64> {
        self.compression_level
    }
    /// Sets the alignment to the given number of bytes.
    #[must_use]
    pub const fn with_alignment(mut self, alignment: u16) -> Self {
        self.alignment = alignment;
        self
    }
}
impl FileOptions<'_, ExtendedFileOptions> {
    /// Adds an extra data field.
    pub fn add_extra_data(
        &mut self,
        header_id: u16,
        data: Box<[u8]>,
        central_only: bool,
    ) -> ZipResult<()> {
        self.extended_options
            .add_extra_data(header_id, data, central_only)
    }

    /// Removes the extra data fields.
    #[must_use]
    pub fn clear_extra_data(mut self) -> Self {
        if !self.extended_options.extra_data.is_empty() {
            self.extended_options.extra_data = Arc::new(vec![]);
        }
        if !self.extended_options.central_extra_data.is_empty() {
            self.extended_options.central_extra_data = Arc::new(vec![]);
        }
        self
    }
}
impl<T: FileOptionExtension> Default for FileOptions<'_, T> {
    /// Construct a new FileOptions object
    fn default() -> Self {
        Self {
            compression_method: Default::default(),
            compression_level: None,
            last_modified_time: DateTime::default_for_write(),
            permissions: None,
            large_file: false,
            encrypt_with: None,
            extended_options: T::default(),
            alignment: 1,
            #[cfg(any())]
            zopfli_buffer_size: Some(1 << 15),
        }
    }
}

impl<W: Write + Seek> Write for ZipWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.writing_to_file {
            return Err(io::Error::other("No file has been started"));
        }
        if buf.is_empty() {
            return Ok(0);
        }
        match self.inner.ref_mut() {
            Some(ref mut w) => {
                let write_result = w.write(buf);
                if let Ok(count) = write_result {
                    self.stats.update(&buf[0..count]);
                    if self.stats.bytes_written > spec::ZIP64_BYTES_THR
                        && !self.files.last_mut().unwrap().1.large_file
                    {
                        let _ = self.abort_file();
                        return Err(io::Error::other("Large file option has not been set"));
                    }
                }
                write_result
            }
            None => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "write(): ZipWriter was already closed",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.ref_mut() {
            Some(ref mut w) => w.flush(),
            None => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "flush(): ZipWriter was already closed",
            )),
        }
    }
}

impl ZipWriterStats {
    fn update(&mut self, buf: &[u8]) {
        self.hasher.update(buf);
        self.bytes_written += buf.len() as u64;
    }
}

impl<A: Read + Write + Seek> ZipWriter<A> {
    /// Initializes the archive from an existing ZIP archive, making it ready for append.
    ///
    /// This uses a default configuration to initially read the archive.
    pub fn new_append(readwriter: A) -> ZipResult<ZipWriter<A>> {
        Self::new_append_with_config(Default::default(), readwriter)
    }

    /// Initializes the archive from an existing ZIP archive, making it ready for append.
    ///
    /// This uses the given read configuration to initially read the archive.
    pub fn new_append_with_config(config: Config, mut readwriter: A) -> ZipResult<ZipWriter<A>> {
        readwriter.seek(SeekFrom::Start(0))?;

        let shared = ZipArchive::get_metadata(config, &mut readwriter)?;

        Ok(ZipWriter {
            inner: Storer(MaybeEncrypted::Unencrypted(readwriter)),
            files: shared.files,
            stats: Default::default(),
            writing_to_file: false,
            comment: shared.comment,
            zip64_comment: shared.zip64_comment,
            writing_raw: true, // avoid recomputing the last file's header
            flush_on_finish_file: false,
        })
    }

    /// `flush_on_finish_file` is designed to support a streaming `inner` that may unload flushed
    /// bytes. It flushes a file's header and body once it starts writing another file. A ZipWriter
    /// will not try to seek back into where a previous file was written unless
    /// either [`ZipWriter::abort_file`] is called while [`ZipWriter::is_writing_file`] returns
    /// false, or [`ZipWriter::deep_copy_file`] is called. In the latter case, it will only need to
    /// read previously-written files and not overwrite them.
    ///
    /// Note: when using an `inner` that cannot overwrite flushed bytes, do not wrap it in a
    /// [BufWriter], because that has a [Seek::seek] method that implicitly calls
    /// [BufWriter::flush], and ZipWriter needs to seek backward to update each file's header with
    /// the size and checksum after writing the body.
    ///
    /// This setting is false by default.
    pub fn set_flush_on_finish_file(&mut self, flush_on_finish_file: bool) {
        self.flush_on_finish_file = flush_on_finish_file;
    }
}

impl<A: Read + Write + Seek> ZipWriter<A> {
    /// Adds another copy of a file already in this archive. This will produce a larger but more
    /// widely-compatible archive compared to a central-directory-only alias. Does not copy alignment.
    pub fn deep_copy_file(&mut self, src_name: &str, dest_name: &str) -> ZipResult<()> {
        self.finish_file()?;
        if src_name == dest_name || self.files.contains_key(dest_name) {
            return Err(invalid!("That file already exists"));
        }
        let write_position = self.inner.get_plain().stream_position()?;
        let src_index = self.index_by_name(src_name)?;
        let src_data = &mut self.files[src_index];
        let src_data_start = src_data.data_start(self.inner.get_plain())?;
        debug_assert!(src_data_start <= write_position);
        let mut compressed_size = src_data.compressed_size;
        if compressed_size > (write_position - src_data_start) {
            compressed_size = write_position - src_data_start;
            src_data.compressed_size = compressed_size;
        }
        let mut reader = BufReader::new(self.inner.get_plain());
        reader.seek(SeekFrom::Start(src_data_start))?;
        let mut copy = vec![0; compressed_size as usize];
        reader.take(compressed_size).read_exact(&mut copy)?;
        self.inner
            .get_plain()
            .seek(SeekFrom::Start(write_position))?;
        let mut new_data = src_data.clone();
        let dest_name_raw = dest_name.as_bytes();
        new_data.file_name = dest_name.into();
        new_data.file_name_raw = dest_name_raw.into();
        new_data.is_utf8 = !dest_name.is_ascii();
        new_data.header_start = write_position;
        let extra_data_start = write_position
            + size_of::<ZipLocalEntryBlock>() as u64
            + new_data.file_name_raw.len() as u64;
        new_data.extra_data_start = Some(extra_data_start);
        let mut data_start = extra_data_start;
        if let Some(extra) = &src_data.extra_field {
            data_start += extra.len() as u64;
        }
        new_data.data_start.take();
        new_data.data_start.get_or_init(|| data_start);
        new_data.central_header_start = 0;
        let block = new_data.local_block()?;
        let index = self.insert_file_data(new_data)?;
        let new_data = &self.files[index];
        let result: io::Result<()> = (|| {
            let plain_writer = self.inner.get_plain();
            block.write(plain_writer)?;
            plain_writer.write_all(&new_data.file_name_raw)?;
            if let Some(data) = &new_data.extra_field {
                plain_writer.write_all(data)?;
            }
            debug_assert_eq!(data_start, plain_writer.stream_position()?);
            self.writing_to_file = true;
            plain_writer.write_all(&copy)?;
            if self.flush_on_finish_file {
                plain_writer.flush()?;
            }
            Ok(())
        })();
        self.ok_or_abort_file(result)?;
        self.writing_to_file = false;
        Ok(())
    }

    /// Like `deep_copy_file`, but uses Path arguments.
    ///
    /// This function ensures that the '/' path separator is used and normalizes `.` and `..`. It
    /// ignores any `..` or Windows drive letter that would produce a path outside the ZIP file's
    /// root.
    pub fn deep_copy_file_from_path<T: AsRef<Path>, U: AsRef<Path>>(
        &mut self,
        src_path: T,
        dest_path: U,
    ) -> ZipResult<()> {
        let src = path_to_string(src_path);
        let dest = path_to_string(dest_path);
        self.deep_copy_file(&src, &dest)
    }

    /// Write the zip file into the backing stream, then produce a readable archive of that data.
    ///
    /// This method avoids parsing the central directory records at the end of the stream for
    /// a slight performance improvement over running [`ZipArchive::new()`] on the output of
    /// [`Self::finish()`].
    ///
    ///```
    /// # fn main() -> Result<(), zip::result::ZipError> {
    /// #
    /// # {
    /// use std::io::{Cursor, prelude::*};
    /// use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
    ///
    /// let buf = Cursor::new(Vec::new());
    /// let mut zip = ZipWriter::new(buf);
    /// let options = SimpleFileOptions::default();
    /// zip.start_file("a.txt", options)?;
    /// zip.write_all(b"hello\n")?;
    ///
    /// let mut zip = zip.finish_into_readable()?;
    /// let mut s: String = String::new();
    /// zip.by_name("a.txt")?.read_to_string(&mut s)?;
    /// assert_eq!(s, "hello\n");
    /// # }
    /// # Ok(())
    /// # }
    ///```
    pub fn finish_into_readable(mut self) -> ZipResult<ZipArchive<A>> {
        let central_start = self.finalize()?;
        let inner = mem::replace(&mut self.inner, Closed).unwrap();
        let comment = mem::take(&mut self.comment);
        let zip64_comment = mem::take(&mut self.zip64_comment);
        let files = mem::take(&mut self.files);

        let archive =
            ZipArchive::from_finalized_writer(files, comment, zip64_comment, inner, central_start)?;
        Ok(archive)
    }
}

impl<W: Write + Seek> ZipWriter<W> {
    /// Initializes the archive.
    ///
    /// Before writing to this object, the [`ZipWriter::start_file`] function should be called.
    /// After a successful write, the file remains open for writing. After a failed write, call
    /// [`ZipWriter::is_writing_file`] to determine if the file remains open.
    pub fn new(inner: W) -> ZipWriter<W> {
        ZipWriter {
            inner: Storer(MaybeEncrypted::Unencrypted(inner)),
            files: IndexMap::new(),
            stats: Default::default(),
            writing_to_file: false,
            writing_raw: false,
            comment: Box::new([]),
            zip64_comment: None,
            flush_on_finish_file: false,
        }
    }

    /// Returns true if a file is currently open for writing.
    pub const fn is_writing_file(&self) -> bool {
        self.writing_to_file && !self.inner.is_closed()
    }

    /// Set ZIP archive comment.
    pub fn set_comment<S>(&mut self, comment: S)
    where
        S: Into<Box<str>>,
    {
        self.set_raw_comment(comment.into().into_boxed_bytes())
    }

    /// Set ZIP archive comment.
    ///
    /// This sets the raw bytes of the comment. The comment
    /// is typically expected to be encoded in UTF-8.
    pub fn set_raw_comment(&mut self, comment: Box<[u8]>) {
        self.comment = comment;
    }

    /// Get ZIP archive comment.
    pub fn get_comment(&mut self) -> Result<&str, Utf8Error> {
        from_utf8(self.get_raw_comment())
    }

    /// Get ZIP archive comment.
    ///
    /// This returns the raw bytes of the comment. The comment
    /// is typically expected to be encoded in UTF-8.
    pub const fn get_raw_comment(&self) -> &[u8] {
        &self.comment
    }

    /// Set ZIP64 archive comment.
    pub fn set_zip64_comment<S>(&mut self, comment: Option<S>)
    where
        S: Into<Box<str>>,
    {
        self.set_raw_zip64_comment(comment.map(|v| v.into().into_boxed_bytes()))
    }

    /// Set ZIP64 archive comment.
    ///
    /// This sets the raw bytes of the comment. The comment
    /// is typically expected to be encoded in UTF-8.
    pub fn set_raw_zip64_comment(&mut self, comment: Option<Box<[u8]>>) {
        self.zip64_comment = comment;
    }

    /// Get ZIP64 archive comment.
    pub fn get_zip64_comment(&mut self) -> Option<Result<&str, Utf8Error>> {
        self.get_raw_zip64_comment().map(from_utf8)
    }

    /// Get ZIP archive comment.
    ///
    /// This returns the raw bytes of the comment. The comment
    /// is typically expected to be encoded in UTF-8.
    pub fn get_raw_zip64_comment(&self) -> Option<&[u8]> {
        self.zip64_comment.as_deref()
    }

    /// Set the file length and crc32 manually.
    ///
    /// # Safety
    ///
    /// This overwrites the internal crc32 calculation. It should only be used in case
    /// the underlying [Write] is written independently, and you need to adjust the zip metadata.
    pub unsafe fn set_file_metadata(&mut self, length: u64, crc32: u32) -> ZipResult<()> {
        if !self.writing_to_file {
            return Err(ZipError::Io(io::Error::other("No file has been started")));
        }
        self.stats.hasher = Hasher::new_with_initial_len(crc32, length);
        self.stats.bytes_written = length;
        Ok(())
    }

    fn ok_or_abort_file<T, E: Into<ZipError>>(&mut self, result: Result<T, E>) -> ZipResult<T> {
        match result {
            Err(e) => {
                let _ = self.abort_file();
                Err(e.into())
            }
            Ok(t) => Ok(t),
        }
    }

    /// Start a new file for with the requested options.
    fn start_entry<S: ToString, T: FileOptionExtension>(
        &mut self,
        name: S,
        options: FileOptions<T>,
        raw_values: Option<ZipRawValues>,
    ) -> ZipResult<()> {
        self.finish_file()?;

        let header_start = self.inner.get_plain().stream_position()?;
        let raw_values = raw_values.unwrap_or(ZipRawValues {
            crc32: 0,
            compressed_size: 0,
            uncompressed_size: 0,
        });

        let mut extra_data = match options.extended_options.extra_data() {
            Some(data) => data.to_vec(),
            None => vec![],
        };
        let central_extra_data = options.extended_options.central_extra_data();
        if let Some(zip64_block) =
            Zip64ExtraFieldBlock::maybe_new(options.large_file, 0, 0, header_start)
        {
            let mut new_extra_data = zip64_block.serialize().into_vec();
            new_extra_data.append(&mut extra_data);
            extra_data = new_extra_data;
        }
        // Write AES encryption extra data.
        #[allow(unused_mut)]
        let mut aes_extra_data_start = 0;
        #[cfg(any())]
        if let Some(EncryptWith::Aes { mode, .. }) = options.encrypt_with {
            let aes_dummy_extra_data =
                vec![0x02, 0x00, 0x41, 0x45, mode as u8, 0x00, 0x00].into_boxed_slice();
            aes_extra_data_start = extra_data.len() as u64;
            ExtendedFileOptions::add_extra_data_unchecked(
                &mut extra_data,
                0x9901,
                aes_dummy_extra_data,
            )?;
        }

        let (compression_method, aes_mode) = match options.encrypt_with {
            #[cfg(any())]
            Some(EncryptWith::Aes { mode, .. }) => (
                CompressionMethod::Aes,
                Some((mode, AesVendorVersion::Ae2, options.compression_method)),
            ),
            _ => (options.compression_method, None),
        };
        let header_end =
            header_start + size_of::<ZipLocalEntryBlock>() as u64 + name.to_string().len() as u64;

        if options.alignment > 1 {
            let extra_data_end = header_end + extra_data.len() as u64;
            let align = options.alignment as u64;
            let unaligned_header_bytes = extra_data_end % align;
            if unaligned_header_bytes != 0 {
                let mut pad_length = (align - unaligned_header_bytes) as usize;
                while pad_length < 6 {
                    pad_length += align as usize;
                }
                // Add an extra field to the extra_data, per APPNOTE 4.6.11
                let mut pad_body = vec![0; pad_length - 4];
                debug_assert!(pad_body.len() >= 2);
                [pad_body[0], pad_body[1]] = options.alignment.to_le_bytes();
                ExtendedFileOptions::add_extra_data_unchecked(
                    &mut extra_data,
                    0xa11e,
                    pad_body.into_boxed_slice(),
                )?;
                debug_assert_eq!((extra_data.len() as u64 + header_end) % align, 0);
            }
        }
        let extra_data_len = extra_data.len();
        if let Some(data) = central_extra_data {
            if extra_data_len + data.len() > u16::MAX as usize {
                return Err(invalid!(
                    "Extra data and central extra data must be less than 64KiB when combined"
                ));
            }
            ExtendedFileOptions::validate_extra_data(data, true)?;
        }
        let mut file = ZipFileData::initialize_local_block(
            name,
            &options,
            raw_values,
            header_start,
            None,
            aes_extra_data_start,
            compression_method,
            aes_mode,
            &extra_data,
        );
        file.version_made_by = file.version_made_by.max(file.version_needed() as u8);
        file.extra_data_start = Some(header_end);
        let index = self.insert_file_data(file)?;
        self.writing_to_file = true;
        let result: ZipResult<()> = (|| {
            ExtendedFileOptions::validate_extra_data(&extra_data, false)?;
            let file = &mut self.files[index];
            let block = file.local_block()?;
            let writer = self.inner.get_plain();
            block.write(writer)?;
            // file name
            writer.write_all(&file.file_name_raw)?;
            if extra_data_len > 0 {
                writer.write_all(&extra_data)?;
                file.extra_field = Some(extra_data.into());
            }
            Ok(())
        })();
        self.ok_or_abort_file(result)?;
        let writer = self.inner.get_plain();
        self.stats.start = writer.stream_position()?;
        match options.encrypt_with {
            #[cfg(any())]
            Some(EncryptWith::Aes { mode, password }) => {
                let aeswriter = AesWriter::new(
                    mem::replace(&mut self.inner, Closed).unwrap(),
                    mode,
                    password.as_bytes(),
                )?;
                self.inner = Storer(MaybeEncrypted::Aes(aeswriter));
            }
            Some(EncryptWith::ZipCrypto(keys, ..)) => {
                let mut zipwriter = crate::office_converter::zip::zipcrypto::ZipCryptoWriter {
                    writer: mem::replace(&mut self.inner, Closed).unwrap(),
                    buffer: vec![],
                    keys,
                };
                self.stats.start = zipwriter.writer.stream_position()?;
                // crypto_header is counted as part of the data
                let crypto_header = [0u8; 12];
                let result = zipwriter.write_all(&crypto_header);
                self.ok_or_abort_file(result)?;
                self.inner = Storer(MaybeEncrypted::ZipCrypto(zipwriter));
            }
            None => {}
        }
        let file = &mut self.files[index];
        debug_assert!(file.data_start.get().is_none());
        file.data_start.get_or_init(|| self.stats.start);
        self.stats.bytes_written = 0;
        self.stats.hasher = Hasher::new();
        Ok(())
    }

    fn insert_file_data(&mut self, file: ZipFileData) -> ZipResult<usize> {
        if self.files.contains_key(&file.file_name) {
            return Err(invalid!("Duplicate filename: {}", file.file_name));
        }
        let name = file.file_name.to_owned();
        self.files.insert(name.clone(), file);
        Ok(self.files.get_index_of(&name).unwrap())
    }

    fn finish_file(&mut self) -> ZipResult<()> {
        if !self.writing_to_file {
            return Ok(());
        }

        let make_plain_writer = self.inner.prepare_next_writer(
            Stored,
            None,
            #[cfg(any())]
            None,
        )?;
        self.inner.switch_to(make_plain_writer)?;
        self.switch_to_non_encrypting_writer()?;
        let writer = self.inner.get_plain();

        if !self.writing_raw {
            let file = match self.files.last_mut() {
                None => return Ok(()),
                Some((_, f)) => f,
            };
            file.uncompressed_size = self.stats.bytes_written;

            let file_end = writer.stream_position()?;
            debug_assert!(file_end >= self.stats.start);
            file.compressed_size = file_end - self.stats.start;
            let mut crc = true;
            if let Some(aes_mode) = &mut file.aes_mode {
                // We prefer using AE-1 which provides an extra CRC check, but for small files we
                // switch to AE-2 to prevent being able to use the CRC value to reconstruct the
                // unencrypted contents.
                //
                // C.f. https://www.winzip.com/en/support/aes-encryption/#crc-faq
                aes_mode.1 = if self.stats.bytes_written < 20 {
                    crc = false;
                    AesVendorVersion::Ae2
                } else {
                    AesVendorVersion::Ae1
                };
            }
            file.crc32 = if crc {
                self.stats.hasher.clone().finalize()
            } else {
                0
            };
            update_aes_extra_data(writer, file)?;
            update_local_file_header(writer, file)?;
            writer.seek(SeekFrom::Start(file_end))?;
        }
        if self.flush_on_finish_file {
            let result = writer.flush();
            self.ok_or_abort_file(result)?;
        }

        self.writing_to_file = false;
        Ok(())
    }

    fn switch_to_non_encrypting_writer(&mut self) -> Result<(), ZipError> {
        match mem::replace(&mut self.inner, Closed) {
            #[cfg(any())]
            Storer(MaybeEncrypted::Aes(writer)) => {
                self.inner = Storer(MaybeEncrypted::Unencrypted(writer.finish()?));
            }
            Storer(MaybeEncrypted::ZipCrypto(writer)) => {
                let crc32 = self.stats.hasher.clone().finalize();
                self.inner = Storer(MaybeEncrypted::Unencrypted(writer.finish(crc32)?))
            }
            Storer(MaybeEncrypted::Unencrypted(w)) => {
                self.inner = Storer(MaybeEncrypted::Unencrypted(w))
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    /// Removes the file currently being written from the archive if there is one, or else removes
    /// the file most recently written.
    pub fn abort_file(&mut self) -> ZipResult<()> {
        let (_, last_file) = self.files.pop().ok_or(ZipError::FileNotFound)?;
        let make_plain_writer = self.inner.prepare_next_writer(
            Stored,
            None,
            #[cfg(any())]
            None,
        )?;
        self.inner.switch_to(make_plain_writer)?;
        self.switch_to_non_encrypting_writer()?;
        // Make sure this is the last file, and that no shallow copies of it remain; otherwise we'd
        // overwrite a valid file and corrupt the archive
        let rewind_safe: bool = match last_file.data_start.get() {
            None => self.files.is_empty(),
            Some(last_file_start) => self.files.values().all(|file| {
                file.data_start
                    .get()
                    .is_some_and(|start| start < last_file_start)
            }),
        };
        if rewind_safe {
            self.inner
                .get_plain()
                .seek(SeekFrom::Start(last_file.header_start))?;
        }
        self.writing_to_file = false;
        Ok(())
    }

    /// Create a file in the archive and start writing its contents. The file must not have the
    /// same name as a file already in the archive.
    ///
    /// The data should be written using the [`Write`] implementation on this [`ZipWriter`]
    pub fn start_file<S: ToString, T: FileOptionExtension>(
        &mut self,
        name: S,
        mut options: FileOptions<T>,
    ) -> ZipResult<()> {
        options.normalize();
        let make_new_self = self.inner.prepare_next_writer(
            options.compression_method,
            options.compression_level,
            #[cfg(any())]
            options.zopfli_buffer_size,
        )?;
        self.start_entry(name, options, None)?;
        let result = self.inner.switch_to(make_new_self);
        self.ok_or_abort_file(result)?;
        self.writing_raw = false;
        Ok(())
    }

    /* TODO: link to/use Self::finish_into_readable() from https://github.com/zip-rs/zip/pull/400 in
     * this docstring. */
    /// Copy over the entire contents of another archive verbatim.
    ///
    /// This method extracts file metadata from the `source` archive, then simply performs a single
    /// big [`io::copy()`](io::copy) to transfer all the actual file contents without any
    /// decompression or decryption. This is more performant than the equivalent operation of
    /// calling [`Self::raw_copy_file()`] for each entry from the `source` archive in sequence.
    ///
    ///```
    /// # fn main() -> Result<(), zip::result::ZipError> {
    /// #
    /// # {
    /// use std::io::{Cursor, prelude::*};
    /// use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};
    ///
    /// let buf = Cursor::new(Vec::new());
    /// let mut zip = ZipWriter::new(buf);
    /// zip.start_file("a.txt", SimpleFileOptions::default())?;
    /// zip.write_all(b"hello\n")?;
    /// let src = ZipArchive::new(zip.finish()?)?;
    ///
    /// let buf = Cursor::new(Vec::new());
    /// let mut zip = ZipWriter::new(buf);
    /// zip.start_file("b.txt", SimpleFileOptions::default())?;
    /// zip.write_all(b"hey\n")?;
    /// let src2 = ZipArchive::new(zip.finish()?)?;
    ///
    /// let buf = Cursor::new(Vec::new());
    ///
    /// let mut zip = ZipWriter::new(buf);
    /// zip.merge_archive(src)?;
    /// zip.merge_archive(src2)?;
    /// let mut result = ZipArchive::new(zip.finish()?)?;
    ///
    /// let mut s: String = String::new();
    /// result.by_name("a.txt")?.read_to_string(&mut s)?;
    /// assert_eq!(s, "hello\n");
    /// s.clear();
    /// result.by_name("b.txt")?.read_to_string(&mut s)?;
    /// assert_eq!(s, "hey\n");
    /// # }
    /// # Ok(())
    /// # }
    ///```
    pub fn merge_archive<R>(&mut self, mut source: ZipArchive<R>) -> ZipResult<()>
    where
        R: Read + Seek,
    {
        self.finish_file()?;

        /* Ensure we accept the file contents on faith (and avoid overwriting the data).
         * See raw_copy_file_rename(). */
        self.writing_to_file = true;
        self.writing_raw = true;

        let writer = self.inner.get_plain();
        /* Get the file entries from the source archive. */
        let new_files = source.merge_contents(writer)?;

        /* These file entries are now ours! */
        self.files.extend(new_files);

        Ok(())
    }

    /// Starts a file, taking a Path as argument.
    ///
    /// This function ensures that the '/' path separator is used and normalizes `.` and `..`. It
    /// ignores any `..` or Windows drive letter that would produce a path outside the ZIP file's
    /// root.
    pub fn start_file_from_path<E: FileOptionExtension, P: AsRef<Path>>(
        &mut self,
        path: P,
        options: FileOptions<E>,
    ) -> ZipResult<()> {
        self.start_file(path_to_string(path), options)
    }

    /// Add a new file using the already compressed data from a ZIP file being read and renames it, this
    /// allows faster copies of the `ZipFile` since there is no need to decompress and compress it again.
    /// Any `ZipFile` metadata is copied and not checked, for example the file CRC.
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write};
    /// use zip::{ZipArchive, ZipWriter};
    ///
    /// fn copy_rename<R, W>(
    ///     src: &mut ZipArchive<R>,
    ///     dst: &mut ZipWriter<W>,
    /// ) -> zip::result::ZipResult<()>
    /// where
    ///     R: Read + Seek,
    ///     W: Write + Seek,
    /// {
    ///     // Retrieve file entry by name
    ///     let file = src.by_name("src_file.txt")?;
    ///
    ///     // Copy and rename the previously obtained file entry to the destination zip archive
    ///     dst.raw_copy_file_rename(file, "new_name.txt")?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn raw_copy_file_rename<R: Read, S: ToString>(
        &mut self,
        file: ZipFile<R>,
        name: S,
    ) -> ZipResult<()> {
        let options = file.options();
        self.raw_copy_file_rename_internal(file, name, options)
    }

    fn raw_copy_file_rename_internal<R: Read, S: ToString>(
        &mut self,
        mut file: ZipFile<R>,
        name: S,
        options: SimpleFileOptions,
    ) -> ZipResult<()> {
        let raw_values = ZipRawValues {
            crc32: file.crc32(),
            compressed_size: file.compressed_size(),
            uncompressed_size: file.size(),
        };

        self.start_entry(name, options, Some(raw_values))?;
        self.writing_to_file = true;
        self.writing_raw = true;

        io::copy(&mut file.take_raw_reader()?, self)?;
        self.finish_file()
    }

    /// Like `raw_copy_file_to_path`, but uses Path arguments.
    ///
    /// This function ensures that the '/' path separator is used and normalizes `.` and `..`. It
    /// ignores any `..` or Windows drive letter that would produce a path outside the ZIP file's
    /// root.
    pub fn raw_copy_file_to_path<R: Read, P: AsRef<Path>>(
        &mut self,
        file: ZipFile<R>,
        path: P,
    ) -> ZipResult<()> {
        self.raw_copy_file_rename(file, path_to_string(path))
    }

    /// Add a new file using the already compressed data from a ZIP file being read, this allows faster
    /// copies of the `ZipFile` since there is no need to decompress and compress it again. Any `ZipFile`
    /// metadata is copied and not checked, for example the file CRC.
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::{Read, Seek, Write};
    /// use zip::{ZipArchive, ZipWriter};
    ///
    /// fn copy<R, W>(src: &mut ZipArchive<R>, dst: &mut ZipWriter<W>) -> zip::result::ZipResult<()>
    /// where
    ///     R: Read + Seek,
    ///     W: Write + Seek,
    /// {
    ///     // Retrieve file entry by name
    ///     let file = src.by_name("src_file.txt")?;
    ///
    ///     // Copy the previously obtained file entry to the destination zip archive
    ///     dst.raw_copy_file(file)?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn raw_copy_file<R: Read>(&mut self, file: ZipFile<R>) -> ZipResult<()> {
        let name = file.name().to_owned();
        self.raw_copy_file_rename(file, name)
    }

    /// Add a new file using the already compressed data from a ZIP file being read and set the last
    /// modified date and unix mode. This allows faster copies of the `ZipFile` since there is no need
    /// to decompress and compress it again. Any `ZipFile` metadata other than the last modified date
    /// and the unix mode is copied and not checked, for example the file CRC.
    ///
    /// ```no_run
    /// use std::io::{Read, Seek, Write};
    /// use zip::{DateTime, ZipArchive, ZipWriter};
    ///
    /// fn copy<R, W>(src: &mut ZipArchive<R>, dst: &mut ZipWriter<W>) -> zip::result::ZipResult<()>
    /// where
    ///     R: Read + Seek,
    ///     W: Write + Seek,
    /// {
    ///     // Retrieve file entry by name
    ///     let file = src.by_name("src_file.txt")?;
    ///
    ///     // Copy the previously obtained file entry to the destination zip archive
    ///     dst.raw_copy_file_touch(file, DateTime::default(), Some(0o644))?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn raw_copy_file_touch<R: Read>(
        &mut self,
        file: ZipFile<R>,
        last_modified_time: DateTime,
        unix_mode: Option<u32>,
    ) -> ZipResult<()> {
        let name = file.name().to_owned();

        let mut options = file.options();

        options = options.last_modified_time(last_modified_time);

        if let Some(perms) = unix_mode {
            options = options.unix_permissions(perms);
        }

        options.normalize();

        self.raw_copy_file_rename_internal(file, name, options)
    }

    /// Add a directory entry.
    ///
    /// As directories have no content, you must not call [`ZipWriter::write`] before adding a new file.
    pub fn add_directory<S, T: FileOptionExtension>(
        &mut self,
        name: S,
        mut options: FileOptions<T>,
    ) -> ZipResult<()>
    where
        S: Into<String>,
    {
        if options.permissions.is_none() {
            options.permissions = Some(0o755);
        }
        *options.permissions.as_mut().unwrap() |= 0o40000;
        options.compression_method = Stored;
        options.encrypt_with = None;

        let name_as_string = name.into();
        // Append a slash to the filename if it does not end with it.
        let name_with_slash = match name_as_string.chars().last() {
            Some('/') | Some('\\') => name_as_string,
            _ => name_as_string + "/",
        };

        self.start_entry(name_with_slash, options, None)?;
        self.writing_to_file = false;
        self.switch_to_non_encrypting_writer()?;
        Ok(())
    }

    /// Add a directory entry, taking a Path as argument.
    ///
    /// This function ensures that the '/' path separator is used and normalizes `.` and `..`. It
    /// ignores any `..` or Windows drive letter that would produce a path outside the ZIP file's
    /// root.
    pub fn add_directory_from_path<T: FileOptionExtension, P: AsRef<Path>>(
        &mut self,
        path: P,
        options: FileOptions<T>,
    ) -> ZipResult<()> {
        self.add_directory(path_to_string(path), options)
    }

    /// Finish the last file and write all other zip-structures
    ///
    /// This will return the writer, but one should normally not append any data to the end of the file.
    /// Note that the zipfile will also be finished on drop.
    pub fn finish(mut self) -> ZipResult<W> {
        let _central_start = self.finalize()?;
        let inner = mem::replace(&mut self.inner, Closed);
        Ok(inner.unwrap())
    }

    /// Add a symlink entry.
    ///
    /// The zip archive will contain an entry for path `name` which is a symlink to `target`.
    ///
    /// No validation or normalization of the paths is performed. For best results,
    /// callers should normalize `\` to `/` and ensure symlinks are relative to other
    /// paths within the zip archive.
    ///
    /// WARNING: not all zip implementations preserve symlinks on extract. Some zip
    /// implementations may materialize a symlink as a regular file, possibly with the
    /// content incorrectly set to the symlink target. For maximum portability, consider
    /// storing a regular file instead.
    pub fn add_symlink<N: ToString, T: ToString, E: FileOptionExtension>(
        &mut self,
        name: N,
        target: T,
        mut options: FileOptions<E>,
    ) -> ZipResult<()> {
        if options.permissions.is_none() {
            options.permissions = Some(0o777);
        }
        *options.permissions.as_mut().unwrap() |= S_IFLNK;
        // The symlink target is stored as file content. And compressing the target path
        // likely wastes space. So always store.
        options.compression_method = Stored;

        self.start_entry(name, options, None)?;
        self.writing_to_file = true;
        let result = self.write_all(target.to_string().as_bytes());
        self.ok_or_abort_file(result)?;
        self.writing_raw = false;
        self.finish_file()?;

        Ok(())
    }

    /// Add a symlink entry, taking Paths to the location and target as arguments.
    ///
    /// This function ensures that the '/' path separator is used and normalizes `.` and `..`. It
    /// ignores any `..` or Windows drive letter that would produce a path outside the ZIP file's
    /// root.
    pub fn add_symlink_from_path<P: AsRef<Path>, T: AsRef<Path>, E: FileOptionExtension>(
        &mut self,
        path: P,
        target: T,
        options: FileOptions<E>,
    ) -> ZipResult<()> {
        self.add_symlink(path_to_string(path), path_to_string(target), options)
    }

    fn finalize(&mut self) -> ZipResult<u64> {
        self.finish_file()?;

        let mut central_start = self.write_central_and_footer()?;
        let writer = self.inner.get_plain();
        let footer_end = writer.stream_position()?;
        let archive_end = writer.seek(SeekFrom::End(0))?;
        if footer_end < archive_end {
            // Data from an aborted file is past the end of the footer.

            // Overwrite the magic so the footer is no longer valid.
            writer.seek(SeekFrom::Start(central_start))?;
            writer.write_u32_le(0)?;
            writer.seek(SeekFrom::Start(
                footer_end - size_of::<Zip32CDEBlock>() as u64 - self.comment.len() as u64,
            ))?;
            writer.write_u32_le(0)?;

            // Rewrite the footer at the actual end.
            let central_and_footer_size = footer_end - central_start;
            writer.seek(SeekFrom::End(-(central_and_footer_size as i64)))?;
            central_start = self.write_central_and_footer()?;
            debug_assert!(self.inner.get_plain().stream_position()? == archive_end);
        }

        Ok(central_start)
    }

    fn write_central_and_footer(&mut self) -> Result<u64, ZipError> {
        let writer = self.inner.get_plain();

        let mut version_needed = MIN_VERSION as u16;
        let central_start = writer.stream_position()?;
        for file in self.files.values() {
            write_central_directory_header(writer, file)?;
            version_needed = version_needed.max(file.version_needed());
        }
        let central_size = writer.stream_position()? - central_start;
        let is64 = self.files.len() > spec::ZIP64_ENTRY_THR
            || central_size.max(central_start) > spec::ZIP64_BYTES_THR
            || self.zip64_comment.is_some();

        if is64 {
            let comment = self.zip64_comment.clone().unwrap_or_default();

            let zip64_footer = spec::Zip64CentralDirectoryEnd {
                record_size: comment.len() as u64 + 44,
                version_made_by: version_needed,
                version_needed_to_extract: version_needed,
                disk_number: 0,
                disk_with_central_directory: 0,
                number_of_files_on_this_disk: self.files.len() as u64,
                number_of_files: self.files.len() as u64,
                central_directory_size: central_size,
                central_directory_offset: central_start,
                extensible_data_sector: comment,
            };

            zip64_footer.write(writer)?;

            let zip64_footer = spec::Zip64CentralDirectoryEndLocator {
                disk_with_central_directory: 0,
                end_of_central_directory_offset: central_start + central_size,
                number_of_disks: 1,
            };

            zip64_footer.write(writer)?;
        }

        let number_of_files = self.files.len().min(spec::ZIP64_ENTRY_THR) as u16;
        let footer = spec::Zip32CentralDirectoryEnd {
            disk_number: 0,
            disk_with_central_directory: 0,
            zip_file_comment: self.comment.clone(),
            number_of_files_on_this_disk: number_of_files,
            number_of_files,
            central_directory_size: central_size.min(spec::ZIP64_BYTES_THR) as u32,
            central_directory_offset: central_start.min(spec::ZIP64_BYTES_THR) as u32,
        };

        footer.write(writer)?;
        Ok(central_start)
    }

    fn index_by_name(&self, name: &str) -> ZipResult<usize> {
        self.files.get_index_of(name).ok_or(ZipError::FileNotFound)
    }
}

impl<W: Write + Seek> Drop for ZipWriter<W> {
    fn drop(&mut self) {
        if !self.inner.is_closed() {
            if let Err(e) = self.finalize() {
                let _ = write!(io::stderr(), "ZipWriter drop failed: {e:?}");
            }
        }
    }
}

type SwitchWriterFunction<W> = Box<dyn FnOnce(MaybeEncrypted<W>) -> GenericZipWriter<W>>;

impl<W: Write + Seek> GenericZipWriter<W> {
    fn prepare_next_writer(
        &self,
        compression: CompressionMethod,
        compression_level: Option<i64>,
        #[cfg(any())] zopfli_buffer_size: Option<usize>,
    ) -> ZipResult<SwitchWriterFunction<W>> {
        if let Closed = self {
            return Err(
                io::Error::new(io::ErrorKind::BrokenPipe, "ZipWriter was already closed").into(),
            );
        }

        {
            #[allow(deprecated)]
            #[allow(unreachable_code)]
            match compression {
                Stored => {
                    if compression_level.is_some() {
                        Err(UnsupportedArchive("Unsupported compression level"))
                    } else {
                        Ok(Box::new(|bare| Storer(bare)))
                    }
                }

                CompressionMethod::Deflated => {
                    let default = Compression::default().level() as i64;

                    #[cfg(any())]
                    let default = 24;

                    let level = clamp_opt(
                        compression_level.unwrap_or(default),
                        deflate_compression_level_range(),
                    )
                    .ok_or(UnsupportedArchive("Unsupported compression level"))?
                        as u32;

                    #[cfg(any())]
                    macro_rules! deflate_zopfli_and_return {
                        ($bare:expr, $best_non_zopfli:expr) => {
                            let options = Options {
                                iteration_count: NonZeroU64::try_from(
                                    (level - $best_non_zopfli) as u64,
                                )
                                .unwrap(),
                                ..Default::default()
                            };
                            return Ok(Box::new(move |bare| match zopfli_buffer_size {
                                Some(size) => GenericZipWriter::BufferedZopfliDeflater(
                                    BufWriter::with_capacity(
                                        size,
                                        zopfli::DeflateEncoder::new(
                                            options,
                                            Default::default(),
                                            bare,
                                        ),
                                    ),
                                ),
                                None => GenericZipWriter::ZopfliDeflater(
                                    zopfli::DeflateEncoder::new(options, Default::default(), bare),
                                ),
                            }));
                        };
                    }

                    #[cfg(any())]
                    {
                        let best_non_zopfli = Compression::best().level();
                        if level > best_non_zopfli {
                            deflate_zopfli_and_return!(bare, best_non_zopfli);
                        }
                    }

                    #[cfg(any())]
                    {
                        let best_non_zopfli = 9;
                        deflate_zopfli_and_return!(bare, best_non_zopfli);
                    }

                    {
                        Ok(Box::new(move |bare| {
                            GenericZipWriter::Deflater(DeflateEncoder::new(
                                bare,
                                Compression::new(level),
                            ))
                        }))
                    }
                }
                #[cfg(any())]
                CompressionMethod::Deflate64 => {
                    Err(UnsupportedArchive("Compressing Deflate64 is not supported"))
                }
                #[cfg(any())]
                CompressionMethod::Bzip2 => {
                    let level = clamp_opt(
                        compression_level.unwrap_or(bzip2::Compression::default().level() as i64),
                        bzip2_compression_level_range(),
                    )
                    .ok_or(UnsupportedArchive("Unsupported compression level"))?
                        as u32;
                    Ok(Box::new(move |bare| {
                        GenericZipWriter::Bzip2(BzEncoder::new(
                            bare,
                            bzip2::Compression::new(level),
                        ))
                    }))
                }
                CompressionMethod::AES => Err(UnsupportedArchive(
                    "AES encryption is enabled through FileOptions::with_aes_encryption",
                )),
                #[cfg(any())]
                CompressionMethod::Zstd => {
                    let level = clamp_opt(
                        compression_level.unwrap_or(zstd::DEFAULT_COMPRESSION_LEVEL as i64),
                        zstd::compression_level_range(),
                    )
                    .ok_or(UnsupportedArchive("Unsupported compression level"))?;
                    Ok(Box::new(move |bare| {
                        GenericZipWriter::Zstd(ZstdEncoder::new(bare, level as i32).unwrap())
                    }))
                }
                #[cfg(any())]
                CompressionMethod::Lzma => {
                    Err(UnsupportedArchive("LZMA isn't supported for compression"))
                }
                #[cfg(any())]
                CompressionMethod::Xz => {
                    let level = clamp_opt(compression_level.unwrap_or(6), 0..=9)
                        .ok_or(UnsupportedArchive("Unsupported compression level"))?
                        as u32;
                    Ok(Box::new(move |bare| {
                        GenericZipWriter::Xz(xz2::write::XzEncoder::new(bare, level))
                    }))
                }
                CompressionMethod::Unsupported(..) => {
                    Err(UnsupportedArchive("Unsupported compression"))
                }
            }
        }
    }

    fn switch_to(&mut self, make_new_self: SwitchWriterFunction<W>) -> ZipResult<()> {
        let bare = match mem::replace(self, Closed) {
            Storer(w) => w,

            GenericZipWriter::Deflater(w) => w.finish()?,
            #[cfg(any())]
            GenericZipWriter::ZopfliDeflater(w) => w.finish()?,
            #[cfg(any())]
            GenericZipWriter::BufferedZopfliDeflater(w) => w
                .into_inner()
                .map_err(|e| ZipError::Io(e.into_error()))?
                .finish()?,
            #[cfg(any())]
            GenericZipWriter::Bzip2(w) => w.finish()?,
            #[cfg(any())]
            GenericZipWriter::Zstd(w) => w.finish()?,
            #[cfg(any())]
            GenericZipWriter::Xz(w) => w.finish()?,
            Closed => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "ZipWriter was already closed",
                )
                .into());
            }
        };
        *self = make_new_self(bare);
        Ok(())
    }

    fn ref_mut(&mut self) -> Option<&mut dyn Write> {
        match self {
            Storer(ref mut w) => Some(w as &mut dyn Write),

            GenericZipWriter::Deflater(ref mut w) => Some(w as &mut dyn Write),
            #[cfg(any())]
            GenericZipWriter::ZopfliDeflater(w) => Some(w as &mut dyn Write),
            #[cfg(any())]
            GenericZipWriter::BufferedZopfliDeflater(w) => Some(w as &mut dyn Write),
            #[cfg(any())]
            GenericZipWriter::Bzip2(ref mut w) => Some(w as &mut dyn Write),
            #[cfg(any())]
            GenericZipWriter::Zstd(ref mut w) => Some(w as &mut dyn Write),
            #[cfg(any())]
            GenericZipWriter::Xz(ref mut w) => Some(w as &mut dyn Write),
            Closed => None,
        }
    }

    const fn is_closed(&self) -> bool {
        matches!(*self, Closed)
    }

    fn get_plain(&mut self) -> &mut W {
        match *self {
            Storer(MaybeEncrypted::Unencrypted(ref mut w)) => w,
            _ => panic!("Should have switched to stored and unencrypted beforehand"),
        }
    }

    fn unwrap(self) -> W {
        match self {
            Storer(MaybeEncrypted::Unencrypted(w)) => w,
            _ => panic!("Should have switched to stored and unencrypted beforehand"),
        }
    }
}

fn deflate_compression_level_range() -> std::ops::RangeInclusive<i64> {
    let min = Compression::fast().level() as i64;
    #[cfg(any())]
    let min = 1;

    #[cfg(any())]
    let max = 264;

    let max = Compression::best().level() as i64;

    min..=max
}

#[cfg(any())]
fn bzip2_compression_level_range() -> std::ops::RangeInclusive<i64> {
    let min = bzip2::Compression::fast().level() as i64;
    let max = bzip2::Compression::best().level() as i64;
    min..=max
}

fn clamp_opt<T: Ord + Copy, U: Ord + Copy + TryFrom<T>>(
    value: T,
    range: std::ops::RangeInclusive<U>,
) -> Option<T> {
    if range.contains(&value.try_into().ok()?) {
        Some(value)
    } else {
        None
    }
}

fn update_aes_extra_data<W: Write + Seek>(writer: &mut W, file: &mut ZipFileData) -> ZipResult<()> {
    let Some((aes_mode, version, compression_method)) = file.aes_mode else {
        return Ok(());
    };

    let extra_data_start = file.extra_data_start.unwrap();

    writer.seek(SeekFrom::Start(
        extra_data_start + file.aes_extra_data_start,
    ))?;

    let mut buf = Vec::new();

    /* TODO: implement this using the Block trait! */
    // Extra field header ID.
    buf.write_u16_le(0x9901)?;
    // Data size.
    buf.write_u16_le(7)?;
    // Integer version number.
    buf.write_u16_le(version as u16)?;
    // Vendor ID.
    buf.write_all(b"AE")?;
    // AES encryption strength.
    buf.write_all(&[aes_mode as u8])?;
    // Real compression method.
    buf.write_u16_le(compression_method.serialize_to_u16())?;

    writer.write_all(&buf)?;

    let aes_extra_data_start = file.aes_extra_data_start as usize;
    let extra_field = Arc::get_mut(file.extra_field.as_mut().unwrap()).unwrap();
    extra_field[aes_extra_data_start..aes_extra_data_start + buf.len()].copy_from_slice(&buf);

    Ok(())
}

fn update_local_file_header<T: Write + Seek>(
    writer: &mut T,
    file: &mut ZipFileData,
) -> ZipResult<()> {
    const CRC32_OFFSET: u64 = 14;
    writer.seek(SeekFrom::Start(file.header_start + CRC32_OFFSET))?;
    writer.write_u32_le(file.crc32)?;
    if file.large_file {
        writer.write_u32_le(spec::ZIP64_BYTES_THR as u32)?;
        writer.write_u32_le(spec::ZIP64_BYTES_THR as u32)?;

        update_local_zip64_extra_field(writer, file)?;

        file.compressed_size = spec::ZIP64_BYTES_THR;
        file.uncompressed_size = spec::ZIP64_BYTES_THR;
    } else {
        // check compressed size as well as it can also be slightly larger than uncompressed size
        if file.compressed_size > spec::ZIP64_BYTES_THR {
            return Err(ZipError::Io(io::Error::other(
                "Large file option has not been set",
            )));
        }
        writer.write_u32_le(file.compressed_size as u32)?;
        // uncompressed size is already checked on write to catch it as soon as possible
        writer.write_u32_le(file.uncompressed_size as u32)?;
    }
    Ok(())
}

fn write_central_directory_header<T: Write>(writer: &mut T, file: &ZipFileData) -> ZipResult<()> {
    let block = file.block()?;
    block.write(writer)?;
    // file name
    writer.write_all(&file.file_name_raw)?;
    // extra field
    if let Some(extra_field) = &file.extra_field {
        writer.write_all(extra_field)?;
    }
    if let Some(central_extra_field) = &file.central_extra_field {
        writer.write_all(central_extra_field)?;
    }
    // file comment
    writer.write_all(file.file_comment.as_bytes())?;

    Ok(())
}

fn update_local_zip64_extra_field<T: Write + Seek>(
    writer: &mut T,
    file: &mut ZipFileData,
) -> ZipResult<()> {
    let block = file.zip64_extra_field_block().ok_or(invalid!(
        "Attempted to update a nonexistent ZIP64 extra field"
    ))?;

    let zip64_extra_field_start = file.header_start
        + size_of::<ZipLocalEntryBlock>() as u64
        + file.file_name_raw.len() as u64;

    writer.seek(SeekFrom::Start(zip64_extra_field_start))?;
    let block = block.serialize();
    writer.write_all(&block)?;

    let extra_field = Arc::get_mut(file.extra_field.as_mut().unwrap()).unwrap();
    extra_field[..block.len()].copy_from_slice(&block);

    Ok(())
}

const EXTRA_FIELD_MAPPING: [u16; 43] = [
    0x0007, 0x0008, 0x0009, 0x000a, 0x000c, 0x000d, 0x000e, 0x000f, 0x0014, 0x0015, 0x0016, 0x0017,
    0x0018, 0x0019, 0x0020, 0x0021, 0x0022, 0x0023, 0x0065, 0x0066, 0x4690, 0x07c8, 0x2605, 0x2705,
    0x2805, 0x334d, 0x4341, 0x4453, 0x4704, 0x470f, 0x4b46, 0x4c41, 0x4d49, 0x4f4c, 0x5356, 0x554e,
    0x5855, 0x6542, 0x756e, 0x7855, 0xa220, 0xfd4a, 0x9902,
];
