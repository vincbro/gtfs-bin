use std::{io, result};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file is too small to contain a valid header")]
    FileTooSmall,
    #[error("section is out of bounds")]
    SectionOutOfBound,
    #[error("section is misaligned or not a whole number of items")]
    InvalidSection,
    #[error("invalid magic number: not a compiled flatnet file")]
    InvalidMagic,
    #[error("unsupported flatnet version: expected {expected}, got {actual}")]
    UnsupportedVersion { expected: u32, actual: u32 },
    #[error("{section} section is not valid utf-8")]
    InvalidUtf8 { section: &'static str },
    #[error("the operating system refused the advice")]
    Advise(#[from] io::Error),
    #[error("file is {actual} bytes but the header says {expected}")]
    LengthMismatch { expected: u64, actual: u64 },
    #[error("the checksum does not match, so the file is damaged")]
    ChecksumMismatch,
    #[error("{table} is not in the order its lookups need")]
    InvalidOrder { table: &'static str },
    #[error("{field} of {table} points outside what it refers to")]
    InvalidReference {
        table: &'static str,
        field: &'static str,
    },
    #[error("{lookup} does not have one entry per item of {table}")]
    MismatchedSections {
        table: &'static str,
        lookup: &'static str,
    },
}

pub type Result<T> = result::Result<T, Error>;
