use std::result;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file is too small to contain a valid header")]
    FileTooSmall,
    #[error("section is out of bounds")]
    SectionOutOfBound,
    #[error("section is misaligned or not a whole number of items")]
    InvalidSection,
    #[error("invalid magic number: not a compiled gtfs file")]
    InvalidMagic,
    #[error("unsupported gtfs binary version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u32, actual: u32 },
    #[error("{section} section is not valid utf-8")]
    InvalidUtf8 { section: &'static str },
    #[error("{lookup} does not have one entry per item of {table}")]
    MismatchedSections {
        table: &'static str,
        lookup: &'static str,
    },
}

pub type Result<T> = result::Result<T, Error>;
