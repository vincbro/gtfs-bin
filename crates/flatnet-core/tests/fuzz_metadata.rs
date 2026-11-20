//! Property tests for the JSON the compiler is given as metadata.
//!
//! Whatever it is given, compiling returns an error or a file that holds the merged object, and
//! it never panics.

mod common;

use std::collections::BTreeMap;

use flatnet_compiler::{Compiler, Error};
use flatnet_core::consumer::Consumer;
use proptest::{collection, prelude::*};
use serde_json::{Map, Value};

fn json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(Value::from),
        any::<String>().prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            collection::btree_map(any::<String>(), inner, 0..4)
                .prop_map(|entries| Value::Object(entries.into_iter().collect())),
        ]
    })
}

fn json_object() -> impl Strategy<Value = Map<String, Value>> {
    collection::btree_map(any::<String>(), json_value(), 0..5)
        .prop_map(|entries| entries.into_iter().collect())
}

fn compile(json: Option<String>, entries: &BTreeMap<String, String>) -> Result<Vec<u8>, Error> {
    let mut compiler = Compiler::new();
    if let Some(json) = json {
        compiler = compiler.metadata_json(json);
    }
    for (key, value) in entries {
        compiler = compiler.metadata_entry(key.clone(), value.clone());
    }
    compiler.compile_gtfs(&common::fixture_path())
}

/// Opens the file and returns the metadata JSON it holds, parsed, or null if it holds none.
fn stored(bytes: &[u8]) -> Value {
    let (_file, mmap) = common::map(bytes).expect("a compiled file is never empty");
    let consumer = Consumer::new(&mmap)
        .map_err(|error| error.to_string())
        .expect("the compiler wrote a file it can't read");
    consumer.metadata_json().map_or(Value::Null, |json| {
        serde_json::from_str(json).expect("the compiler wrote json that is not json")
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn an_object_and_entries_come_back_merged(
        object in json_object(),
        entries in collection::btree_map(any::<String>(), any::<String>(), 0..4),
    ) {
        let json = serde_json::to_string(&Value::Object(object.clone()))
            .expect("a value is json");
        let mut expected = object;
        for (key, value) in &entries {
            expected.insert(key.clone(), Value::String(value.clone()));
        }

        let bytes = compile(Some(json), &entries).expect("an object is accepted");

        let stored = stored(&bytes);
        if expected.is_empty() {
            prop_assert_eq!(stored, Value::Null, "an empty object is left out");
        } else {
            prop_assert_eq!(stored, Value::Object(expected));
        }
    }

    #[test]
    fn arbitrary_text_is_an_error_unless_it_is_an_object(text in ".*") {
        let is_object = matches!(serde_json::from_str::<Value>(&text), Ok(Value::Object(_)));

        let result = compile(Some(text), &BTreeMap::new());

        prop_assert_eq!(result.is_ok(), is_object, "accepted exactly when it is a json object");
        if let Ok(bytes) = result {
            let value = stored(&bytes);
            prop_assert!(matches!(value, Value::Object(_) | Value::Null), "the file holds an object");
        }
    }

    #[test]
    fn entries_alone_come_back_as_text(
        entries in collection::btree_map(any::<String>(), any::<String>(), 1..5),
    ) {
        let bytes = compile(None, &entries).expect("entries are accepted");

        let expected: Map<String, Value> = entries
            .into_iter()
            .map(|(key, value)| (key, Value::String(value)))
            .collect();
        prop_assert_eq!(stored(&bytes), Value::Object(expected));
    }
}
