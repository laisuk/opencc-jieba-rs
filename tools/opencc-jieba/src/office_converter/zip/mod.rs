//! Private ZIP 3.0 implementation; see ../VENDOR_NOTES.md.
#![allow(dead_code)]
pub use crate::office_converter::zip::compression::CompressionMethod;
pub use crate::office_converter::zip::read::ZipArchive;
pub use crate::office_converter::zip::spec::ZIP64_BYTES_THR;
pub use crate::office_converter::zip::write::ZipWriter;

mod compression;
mod cp437;
mod crc32;
pub mod extra_fields;
mod path;
pub mod read;
pub mod result;
mod spec;
mod types;
pub mod write;
mod zipcrypto;

pub mod unstable;

mod validation;

pub(super) fn unsafe_path(name: &str) -> bool {
    validation::unsafe_path(name)
}

#[cfg(test)]
mod tests;
