use std::{io, path::PathBuf, result};

use zip::result::ZipError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read the zip archive")]
    Zip(#[from] ZipError),
    #[error("{file} is not valid csv")]
    Csv {
        file: &'static str,
        #[source]
        source: csv::Error,
    },
    #[error("the feed has no {0}")]
    MissingFile(&'static str),
    #[error("{file} has no {column} column")]
    MissingColumn {
        file: &'static str,
        column: &'static str,
    },
    #[error("{file} line {line}: invalid {column} {value:?}")]
    InvalidValue {
        file: &'static str,
        line: u64,
        column: &'static str,
        value: String,
    },
    #[error("the metadata is not valid json")]
    InvalidMetadata(#[source] serde_json::Error),
    #[error("the metadata json is not an object")]
    MetadataNotAnObject,
    #[error("service {service_id:?} ends before it starts")]
    ReversedServiceRange { service_id: String },
    #[error("service {service_id:?} runs for {days} days, more than the {maximum} allowed")]
    ServiceRangeTooLong {
        service_id: String,
        days: u32,
        maximum: u32,
    },
}

pub type Result<T> = result::Result<T, Error>;
