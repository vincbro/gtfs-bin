use std::result;
#[cfg(feature = "gtfs")]
use std::{io, path::PathBuf};

#[cfg(feature = "gtfs")]
use zip::result::ZipError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "gtfs")]
    #[error("failed to read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[cfg(feature = "gtfs")]
    #[error("failed to read the zip archive")]
    Zip(#[from] ZipError),
    #[cfg(feature = "gtfs")]
    #[error("{file} is not valid csv")]
    Csv {
        file: &'static str,
        #[source]
        source: csv::Error,
    },
    #[cfg(feature = "gtfs")]
    #[error("the feed has no {0}")]
    MissingFile(&'static str),
    #[cfg(feature = "gtfs")]
    #[error("{file} has no {column} column")]
    MissingColumn {
        file: &'static str,
        column: &'static str,
    },
    #[cfg(feature = "gtfs")]
    #[error("{file} line {line}: invalid {column} {value:?}")]
    InvalidValue {
        file: &'static str,
        line: u64,
        column: &'static str,
        value: String,
    },
    #[cfg(feature = "gtfs")]
    #[error("service {service_id:?} ends before it starts")]
    ReversedServiceRange { service_id: String },
    #[error("the metadata is not valid json")]
    InvalidMetadata(#[source] serde_json::Error),
    #[error("the metadata json is not an object")]
    MetadataNotAnObject,
    #[error("service {service_id:?} runs for {days} days, more than the {maximum} allowed")]
    ServiceRangeTooLong {
        service_id: String,
        days: u32,
        maximum: u32,
    },
    #[error("the network has two {table} with the id {id:?}")]
    DuplicateId { table: &'static str, id: String },
    #[error("{field} of the network's {table} points at nothing")]
    InvalidReference {
        table: &'static str,
        field: &'static str,
    },
    #[error("search needs the text part, because a place is found by its name")]
    SearchWithoutText,
    #[error("the network's service {service_id:?} has no days")]
    EmptyService { service_id: String },
}

pub type Result<T> = result::Result<T, Error>;
