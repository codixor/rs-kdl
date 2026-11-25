#![cfg(feature = "toml-interop")]

use rs_kdl::{
    Document, InteropDocument, InteropRoot, InteropTemporal, InteropTemporalKind, InteropValue,
    Node, TomlInteropError,
};
use rs_toml::{Arena, parse as parse_toml};

const TOML_SAMPLE: &str = r#"
title = "rust-misc"
enabled = true
retries = 3
ratio = 1.0
published = 2026-03-31T08:15:00Z
local_date = 2026-03-31
local_time = 08:15:00
routes = ["/", "/dashboard"]

[[servers]]
name = "api"
port = 8080

[[servers]]
name = "worker"
port = 9090
"#;

#[test]
fn toml_text_round_trip_through_direct_interop() {
    let interop = InteropDocument::from_toml_str(TOML_SAMPLE).expect("toml should parse");
    let rendered = interop
        .to_toml_string()
        .expect("interop map-root should render");
    let reparsed =
        InteropDocument::from_toml_str(&rendered).expect("rendered toml should parse again");

    assert_eq!(reparsed, interop);
}

#[test]
fn toml_document_and_table_bridge_match_text_bridge() {
    let arena = Arena::new();
    let document = parse_toml(TOML_SAMPLE, &arena).expect("sample should parse");
    let from_document = InteropDocument::from_toml_document(&document);
    let from_table = InteropDocument::from_toml_table(document.table());
    let from_text = InteropDocument::from_toml_str(TOML_SAMPLE).expect("text bridge should work");

    assert_eq!(from_document, from_table);
    assert_eq!(from_document, from_text);
}

#[test]
fn toml_text_round_trip_through_kdl_contract_document() {
    let document = Document::from_toml_str(TOML_SAMPLE).expect("toml should convert to KDL");
    let rendered = document
        .to_toml_string()
        .expect("contract document should render back to toml");

    let baseline = InteropDocument::from_toml_str(TOML_SAMPLE).expect("baseline should parse");
    let round_trip =
        InteropDocument::from_toml_str(&rendered).expect("rendered TOML should parse again");

    assert_eq!(round_trip, baseline);
}

#[test]
fn toml_temporals_are_preserved_in_the_neutral_model() {
    let interop = InteropDocument::from_toml_str(TOML_SAMPLE).expect("sample should parse");
    let InteropRoot::Value(InteropValue::Map(entries)) = &interop.root else {
        panic!("TOML interop should be a map-root document");
    };

    let published = entries
        .iter()
        .find(|entry| entry.name.as_str() == "published")
        .expect("published entry should exist");
    let local_date = entries
        .iter()
        .find(|entry| entry.name.as_str() == "local_date")
        .expect("local_date entry should exist");
    let local_time = entries
        .iter()
        .find(|entry| entry.name.as_str() == "local_time")
        .expect("local_time entry should exist");

    assert_eq!(
        published.value,
        InteropValue::Temporal(InteropTemporal::new(
            InteropTemporalKind::OffsetDateTime,
            "2026-03-31T08:15:00Z"
        ))
    );
    assert_eq!(
        local_date.value,
        InteropValue::Temporal(InteropTemporal::new(
            InteropTemporalKind::LocalDate,
            "2026-03-31"
        ))
    );
    assert_eq!(
        local_time.value,
        InteropValue::Temporal(InteropTemporal::new(
            InteropTemporalKind::LocalTime,
            "08:15:00"
        ))
    );
}

#[test]
fn toml_contract_bridge_preserves_number_lexemes() {
    let sample = "retries = 3\nratio = 1.0\n";
    let document = Document::from_toml_str(sample).expect("toml should convert to KDL");
    let direct = InteropDocument::from_toml_str(sample).expect("toml should parse directly");
    let contract = document
        .to_toml_interop()
        .expect("contract document should decode back into neutral TOML");

    assert_eq!(contract, direct);
}

#[test]
fn toml_scalar_root_is_rejected_by_text_rendering() {
    let error = InteropDocument::value(InteropValue::String("hello".into()))
        .to_toml_string()
        .expect_err("toml text must be map-root");

    assert!(matches!(error, TomlInteropError::ExpectedMapRoot));
}

#[test]
fn toml_null_values_are_rejected_by_text_rendering() {
    let error = InteropDocument::value(InteropValue::Map(vec![
        ("nothing".into(), InteropValue::Null).into(),
    ]))
    .to_toml_string()
    .expect_err("TOML does not support null");

    assert!(matches!(error, TomlInteropError::NullUnsupported));
}

#[test]
fn invalid_toml_contract_document_is_rejected() {
    let invalid = Document::from(vec![Node::new("toml-entry")]);

    let error = invalid
        .to_toml_string()
        .expect_err("invalid contract document should fail");

    assert!(matches!(
        error,
        TomlInteropError::InvalidContract("unexpected node name in TOML contract document")
    ));
}
