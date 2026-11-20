use std::result;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the bytes are not a gtfs-realtime feed")]
    Decode(#[from] prost::DecodeError),
    #[error("{0:?} is not a known time zone")]
    UnknownTimeZone(String),
}

pub type Result<T> = result::Result<T, Error>;
