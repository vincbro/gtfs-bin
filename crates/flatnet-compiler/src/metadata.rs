use std::collections::BTreeMap;

use flatnet_core::models::{
    Date, Metadata, Sentinel, Service, Settings, SliceBuilder, StringSlice,
};
use serde_json::{Map, Value};

use crate::{Error, Result, network::NetworkInfo};

/// What the caller gave the compiler to put in the file.
#[derive(Debug, Default, Clone)]
pub(super) struct MetadataInput {
    pub(super) compiled_at: u64,
    pub(super) source: Option<String>,
    pub(super) json: Option<String>,
    pub(super) entries: BTreeMap<String, String>,
}

/// Builds the fixed fields, with their text added to `strings`, and the JSON section.
pub(super) fn build_metadata(
    input: &MetadataInput,
    info: &NetworkInfo,
    settings: Settings,
    services: &[Service],
    strings: &mut SliceBuilder<StringSlice>,
) -> Result<(Metadata, String)> {
    let mut text = |value: Option<&str>| value.map(|value| strings.add(value)).into();
    let (active_from, active_until) = active_range(services);
    let metadata = Metadata {
        compiled_at: input.compiled_at,
        active_from,
        active_until,
        compiler_version: text(Some(env!("CARGO_PKG_VERSION"))),
        source: text(input.source.as_deref()),
        publisher_name: text(info.publisher_name.as_deref()),
        publisher_url: text(info.publisher_url.as_deref()),
        language: text(info.language.as_deref()),
        feed_version: text(info.version.as_deref()),
        contact_email: text(info.contact_email.as_deref()),
        agency_name: text(info.agency_name.as_deref()),
        agency_url: text(info.agency_url.as_deref()),
        settings,
    };
    Ok((metadata, build_json(input)?))
}

/// The first and last day any service runs.
fn active_range(services: &[Service]) -> (Date, Date) {
    let first = services.iter().map(|service| service.start_date).min();
    let last = services.iter().map(|service| service.end_date).max();
    match (first, last) {
        (Some(first), Some(last)) => (first, last),
        _ => (Date::NONE, Date::NONE),
    }
}

/// Merges the JSON object and the single entries into one compact object. Keys are sorted, so the
/// same input gives the same bytes. An entry replaces a key of the same name in the object.
fn build_json(input: &MetadataInput) -> Result<String> {
    let mut object = match &input.json {
        Some(json) => match serde_json::from_str(json).map_err(Error::InvalidMetadata)? {
            Value::Object(object) => object,
            _ => return Err(Error::MetadataNotAnObject),
        },
        None => Map::new(),
    };
    for (key, value) in &input.entries {
        object.insert(key.clone(), Value::String(value.clone()));
    }
    if object.is_empty() {
        return Ok(String::new());
    }
    serde_json::to_string(&object).map_err(Error::InvalidMetadata)
}
